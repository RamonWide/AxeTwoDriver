//! A USB function opened through Microsoft's WinUSB driver: control
//! transfers, alternate settings and queued isochronous transfers.

use crate::{wide, WinError};
use std::sync::Arc;
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_Device_Interface_ListW, CM_Get_Device_Interface_List_SizeW, CM_GET_DEVICE_INTERFACE_LIST_PRESENT, CR_SUCCESS,
};
use windows_sys::Win32::Devices::Usb::{
    WinUsb_AbortPipe, WinUsb_ControlTransfer, WinUsb_Free, WinUsb_GetAssociatedInterface, WinUsb_GetOverlappedResult,
    WinUsb_Initialize, WinUsb_QueryPipeEx, WinUsb_ReadIsochPipeAsap, WinUsb_RegisterIsochBuffer,
    WinUsb_SetCurrentAlternateSetting, WinUsb_UnregisterIsochBuffer, WinUsb_WriteIsochPipeAsap,
    USBD_ISO_PACKET_DESCRIPTOR, WINUSB_PIPE_INFORMATION_EX, WINUSB_SETUP_PACKET,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_IO_PENDING, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
use windows_sys::Win32::System::IO::OVERLAPPED;

type InterfaceHandle = *mut core::ffi::c_void;

fn last_error(what: &'static str) -> WinError {
    // SAFETY: GetLastError has no preconditions.
    let code = unsafe { GetLastError() };
    WinError { what, code }
}

/// Parses `{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}`.
fn parse_guid(text: &str) -> Option<windows_sys::core::GUID> {
    let hex: String = text.chars().filter(char::is_ascii_hexdigit).collect();
    if hex.len() != 32 {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok();
    let mut data4 = [0u8; 8];
    for (i, b) in data4.iter_mut().enumerate() {
        *b = byte(8 + i)?;
    }
    Some(windows_sys::core::GUID {
        data1: u32::from_str_radix(hex.get(0..8)?, 16).ok()?,
        data2: u16::from_str_radix(hex.get(8..12)?, 16).ok()?,
        data3: u16::from_str_radix(hex.get(12..16)?, 16).ok()?,
        data4,
    })
}

/// Paths of the present devices that expose the interface class `guid`.
pub fn interface_paths(guid: &str) -> Result<Vec<String>, WinError> {
    let guid = parse_guid(guid).ok_or(WinError { what: "parsing the interface GUID", code: 0 })?;
    let mut len: u32 = 0;
    // SAFETY: `len` is a valid out pointer and `guid` outlives the call.
    let cr = unsafe {
        CM_Get_Device_Interface_List_SizeW(&mut len, &guid, std::ptr::null(), CM_GET_DEVICE_INTERFACE_LIST_PRESENT)
    };
    if cr != CR_SUCCESS {
        return Err(WinError { what: "listing device interfaces", code: cr });
    }
    let mut buf = vec![0u16; len as usize];
    // SAFETY: `buf` holds exactly `len` UTF-16 units, as the size call asked for.
    let cr = unsafe {
        CM_Get_Device_Interface_ListW(
            &guid,
            std::ptr::null(),
            buf.as_mut_ptr(),
            len,
            CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
        )
    };
    if cr != CR_SUCCESS {
        return Err(WinError { what: "listing device interfaces", code: cr });
    }
    Ok(buf.split(|&c| c == 0).filter(|s| !s.is_empty()).map(String::from_utf16_lossy).collect())
}

struct Inner {
    file: HANDLE,
    /// Index 0 is the function's first interface; the rest are its associated interfaces.
    interfaces: Vec<InterfaceHandle>,
}

// SAFETY: the handles are plain kernel object references; WinUSB allows calls
// on them from any thread, and `Inner` has no interior mutability.
unsafe impl Send for Inner {}
// SAFETY: as above.
unsafe impl Sync for Inner {}

impl Drop for Inner {
    fn drop(&mut self) {
        for &h in self.interfaces.iter().rev() {
            // SAFETY: each handle came from WinUsb_Initialize or
            // WinUsb_GetAssociatedInterface and is freed once.
            unsafe { WinUsb_Free(h) };
        }
        // SAFETY: the file handle came from CreateFileW and is closed once.
        unsafe { CloseHandle(self.file) };
    }
}

/// One endpoint of an interface's alternate setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipeInfo {
    /// Endpoint address, with bit 7 set for IN.
    pub id: u8,
    /// 0 control, 1 isochronous, 2 bulk, 3 interrupt.
    pub kind: u8,
    pub max_packet: u16,
    pub interval: u8,
    pub max_bytes_per_interval: u32,
}

/// A USB function opened through WinUSB.
#[derive(Clone)]
pub struct Device {
    inner: Arc<Inner>,
}

impl Device {
    /// Opens the device at `path` (from [`interface_paths`]).
    pub fn open(path: &str) -> Result<Self, WinError> {
        let wpath = wide(path);
        // SAFETY: `wpath` is NUL-terminated and outlives the call.
        let file = unsafe {
            CreateFileW(
                wpath.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                std::ptr::null_mut(),
            )
        };
        if file == INVALID_HANDLE_VALUE {
            return Err(last_error("opening the device (is another program using it?)"));
        }
        let mut first: InterfaceHandle = std::ptr::null_mut();
        // SAFETY: `file` is a live handle and `first` is a valid out pointer.
        if unsafe { WinUsb_Initialize(file, &mut first) } == 0 {
            let e = last_error("starting WinUSB on the device");
            // SAFETY: `file` is live and closed once on this path.
            unsafe { CloseHandle(file) };
            return Err(e);
        }
        let mut inner = Inner { file, interfaces: vec![first] };
        for index in 0..=u8::MAX {
            let mut next: InterfaceHandle = std::ptr::null_mut();
            // SAFETY: `first` is a live interface handle and `next` a valid out pointer.
            if unsafe { WinUsb_GetAssociatedInterface(first, index, &mut next) } == 0 {
                break;
            }
            inner.interfaces.push(next);
        }
        Ok(Self { inner: Arc::new(inner) })
    }

    /// Number of interfaces in the function.
    pub fn interface_count(&self) -> usize {
        self.inner.interfaces.len()
    }

    fn interface(&self, index: usize) -> Result<InterfaceHandle, WinError> {
        self.inner
            .interfaces
            .get(index)
            .copied()
            .ok_or(WinError { what: "selecting an interface the device does not have", code: 0 })
    }

    fn control(
        &self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: &mut [u8],
    ) -> Result<usize, WinError> {
        let setup = WINUSB_SETUP_PACKET {
            RequestType: request_type,
            Request: request,
            Value: value,
            Index: index,
            Length: data.len() as u16,
        };
        let mut done: u32 = 0;
        // SAFETY: `data` is valid for reads and writes of its length; a null
        // OVERLAPPED makes the call synchronous.
        let ok = unsafe {
            WinUsb_ControlTransfer(
                self.interface(0)?,
                setup,
                data.as_mut_ptr(),
                data.len() as u32,
                &mut done,
                std::ptr::null(),
            )
        };
        if ok == 0 {
            return Err(last_error("a control transfer"));
        }
        Ok((done as usize).min(data.len()))
    }

    /// A device-to-host control transfer; returns the bytes received.
    pub fn control_in(
        &self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        max_len: u16,
    ) -> Result<Vec<u8>, WinError> {
        let mut data = vec![0u8; usize::from(max_len)];
        let n = self.control(request_type | 0x80, request, value, index, &mut data)?;
        data.truncate(n);
        Ok(data)
    }

    /// A host-to-device control transfer.
    pub fn control_out(
        &self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: &[u8],
    ) -> Result<(), WinError> {
        let mut copy = data.to_vec();
        self.control(request_type & 0x7f, request, value, index, &mut copy).map(|_| ())
    }

    /// Selects an alternate setting of the interface at `interface` (position
    /// within the function, not the interface number).
    pub fn set_alternate(&self, interface: usize, alternate: u8) -> Result<(), WinError> {
        // SAFETY: the handle is live for as long as `self` is.
        if unsafe { WinUsb_SetCurrentAlternateSetting(self.interface(interface)?, alternate) } == 0 {
            return Err(last_error("selecting an alternate setting"));
        }
        Ok(())
    }

    /// Describes endpoint number `pipe_index` of an alternate setting.
    pub fn pipe(&self, interface: usize, alternate: u8, pipe_index: u8) -> Result<PipeInfo, WinError> {
        // SAFETY: an all-zero WINUSB_PIPE_INFORMATION_EX is a valid value.
        let mut info: WINUSB_PIPE_INFORMATION_EX = unsafe { std::mem::zeroed() };
        // SAFETY: the handle is live and `info` is a valid out pointer.
        let ok = unsafe { WinUsb_QueryPipeEx(self.interface(interface)?, alternate, pipe_index, &mut info) };
        if ok == 0 {
            return Err(last_error("reading endpoint information"));
        }
        Ok(PipeInfo {
            id: info.PipeId,
            kind: info.PipeType as u8,
            max_packet: info.MaximumPacketSize,
            interval: info.Interval,
            max_bytes_per_interval: info.MaximumBytesPerInterval,
        })
    }

    /// Creates a queue of `slots` isochronous transfers of `transfer_bytes`
    /// each on endpoint `pipe_id`. The interface's streaming alternate setting
    /// must already be selected.
    pub fn iso_queue(
        &self,
        interface: usize,
        pipe_id: u8,
        transfer_bytes: usize,
        packets_per_transfer: usize,
        slots: usize,
    ) -> Result<IsoQueue, WinError> {
        let handle = self.interface(interface)?;
        let mut buffer = vec![0u8; transfer_bytes * slots].into_boxed_slice();
        let mut registration = std::ptr::null_mut();
        // SAFETY: `buffer` is heap memory that stays at the same address and is
        // owned by the queue until after the registration is released in Drop.
        let ok = unsafe {
            WinUsb_RegisterIsochBuffer(handle, pipe_id, buffer.as_mut_ptr(), buffer.len() as u32, &mut registration)
        };
        if ok == 0 {
            return Err(last_error("registering an isochronous buffer"));
        }
        let mut queue = IsoQueue {
            _device: self.inner.clone(),
            interface: handle,
            pipe_id,
            buffer,
            registration,
            transfer_bytes,
            slots: Vec::with_capacity(slots),
            started: false,
        };
        for _ in 0..slots {
            // SAFETY: null attributes and name are allowed; manual reset, initially unset.
            let event = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
            if event.is_null() {
                return Err(last_error("creating an event"));
            }
            queue.slots.push(Slot {
                // SAFETY: an all-zero OVERLAPPED is a valid value.
                overlapped: Box::new(unsafe { std::mem::zeroed() }),
                event,
                packets: vec![USBD_ISO_PACKET_DESCRIPTOR { Offset: 0, Length: 0, Status: 0 }; packets_per_transfer]
                    .into_boxed_slice(),
                pending: false,
            });
        }
        Ok(queue)
    }
}

struct Slot {
    overlapped: Box<OVERLAPPED>,
    event: HANDLE,
    packets: Box<[USBD_ISO_PACKET_DESCRIPTOR]>,
    pending: bool,
}

/// One packet of a completed isochronous read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsoPacket {
    /// Offset within the slot's buffer.
    pub offset: usize,
    pub length: usize,
    /// `USBD_STATUS`; zero is success.
    pub status: i32,
}

/// A fixed set of isochronous transfers on one endpoint, each with its own
/// region of a registered buffer. A slot is either idle (its buffer may be
/// read and written) or pending (the host controller owns it).
pub struct IsoQueue {
    _device: Arc<Inner>,
    interface: InterfaceHandle,
    pipe_id: u8,
    buffer: Box<[u8]>,
    registration: *mut core::ffi::c_void,
    transfer_bytes: usize,
    slots: Vec<Slot>,
    started: bool,
}

// SAFETY: the raw handles are kernel object references usable from any
// thread; all mutation goes through `&mut self`.
unsafe impl Send for IsoQueue {}

const BUSY: WinError = WinError { what: "using a transfer that is still in flight", code: 0 };
const NO_SLOT: WinError = WinError { what: "selecting a transfer slot", code: 0 };

impl IsoQueue {
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    pub fn transfer_bytes(&self) -> usize {
        self.transfer_bytes
    }

    fn idle(&self, slot: usize) -> Result<(), WinError> {
        match self.slots.get(slot) {
            Some(s) if !s.pending => Ok(()),
            Some(_) => Err(BUSY),
            None => Err(NO_SLOT),
        }
    }

    /// The buffer of an idle slot.
    pub fn data(&self, slot: usize) -> Result<&[u8], WinError> {
        self.idle(slot)?;
        let start = slot * self.transfer_bytes;
        self.buffer.get(start..start + self.transfer_bytes).ok_or(NO_SLOT)
    }

    /// The buffer of an idle slot, for filling before a write.
    pub fn data_mut(&mut self, slot: usize) -> Result<&mut [u8], WinError> {
        self.idle(slot)?;
        let start = slot * self.transfer_bytes;
        self.buffer.get_mut(start..start + self.transfer_bytes).ok_or(NO_SLOT)
    }

    /// Packets of the last completed read on an idle slot.
    pub fn packets(&self, slot: usize) -> Result<Vec<IsoPacket>, WinError> {
        self.idle(slot)?;
        let s = self.slots.get(slot).ok_or(NO_SLOT)?;
        Ok(s.packets
            .iter()
            .map(|p| IsoPacket { offset: p.Offset as usize, length: p.Length as usize, status: p.Status })
            .collect())
    }

    fn arm(&mut self, slot: usize) -> Result<*const OVERLAPPED, WinError> {
        self.idle(slot)?;
        let s = self.slots.get_mut(slot).ok_or(NO_SLOT)?;
        // SAFETY: an all-zero OVERLAPPED is a valid value.
        *s.overlapped = unsafe { std::mem::zeroed() };
        s.overlapped.hEvent = s.event;
        Ok(&*s.overlapped as *const OVERLAPPED)
    }

    fn submitted(&mut self, slot: usize, ok: i32, what: &'static str) -> Result<(), WinError> {
        if ok == 0 {
            let e = last_error(what);
            if e.code != ERROR_IO_PENDING {
                return Err(e);
            }
        }
        if let Some(s) = self.slots.get_mut(slot) {
            s.pending = true;
        }
        self.started = true;
        Ok(())
    }

    /// Queues a read that fills the whole slot, to start as soon as possible
    /// after the transfers already queued.
    pub fn submit_read(&mut self, slot: usize) -> Result<(), WinError> {
        let overlapped = self.arm(slot)?;
        let s = self.slots.get_mut(slot).ok_or(NO_SLOT)?;
        let packets = s.packets.as_mut_ptr();
        let count = s.packets.len() as u32;
        // SAFETY: the registered buffer, the packet array and the OVERLAPPED
        // are heap allocations owned by `self`; the slot is marked pending
        // below so none of them is touched or freed until the transfer has
        // completed (Drop aborts and waits).
        let ok = unsafe {
            WinUsb_ReadIsochPipeAsap(
                self.registration,
                (slot * self.transfer_bytes) as u32,
                self.transfer_bytes as u32,
                i32::from(self.started),
                count,
                packets,
                overlapped,
            )
        };
        self.submitted(slot, ok, "queueing an isochronous read")
    }

    /// Queues a write of the first `len` bytes of the slot.
    pub fn submit_write(&mut self, slot: usize, len: usize) -> Result<(), WinError> {
        if len > self.transfer_bytes {
            return Err(WinError { what: "queueing more bytes than a transfer holds", code: 0 });
        }
        let overlapped = self.arm(slot)?;
        // SAFETY: as in `submit_read`.
        let ok = unsafe {
            WinUsb_WriteIsochPipeAsap(
                self.registration,
                (slot * self.transfer_bytes) as u32,
                len as u32,
                i32::from(self.started),
                overlapped,
            )
        };
        self.submitted(slot, ok, "queueing an isochronous write")
    }

    /// Forgets the stream position so the next transfer starts a new stream.
    /// Call after an error broke the sequence.
    pub fn restart(&mut self) {
        self.started = false;
    }

    /// Waits for a pending slot to complete; returns the bytes transferred,
    /// or `None` when `timeout_ms` passed first (the slot stays pending).
    pub fn wait(&mut self, slot: usize, timeout_ms: u32) -> Result<Option<usize>, WinError> {
        let interface = self.interface;
        let s = self.slots.get_mut(slot).ok_or(NO_SLOT)?;
        if !s.pending {
            return Err(WinError { what: "waiting for a transfer that was not queued", code: 0 });
        }
        // SAFETY: `event` is a live event handle owned by the slot.
        match unsafe { WaitForSingleObject(s.event, timeout_ms) } {
            WAIT_OBJECT_0 => {}
            WAIT_TIMEOUT => return Ok(None),
            _ => return Err(last_error("waiting for a transfer")),
        }
        let mut done: u32 = 0;
        // SAFETY: the OVERLAPPED belongs to this slot's signalled transfer.
        let ok = unsafe { WinUsb_GetOverlappedResult(interface, &*s.overlapped, &mut done, 0) };
        s.pending = false;
        if ok == 0 {
            return Err(last_error("an isochronous transfer"));
        }
        Ok(Some(done as usize))
    }
}

impl Drop for IsoQueue {
    fn drop(&mut self) {
        // SAFETY: the interface handle is live because `_device` is still held.
        unsafe { WinUsb_AbortPipe(self.interface, self.pipe_id) };
        for s in &mut self.slots {
            if s.pending {
                let mut done: u32 = 0;
                // SAFETY: waits for the aborted transfer so the kernel no
                // longer references the buffer, packets or OVERLAPPED.
                unsafe { WinUsb_GetOverlappedResult(self.interface, &*s.overlapped, &mut done, 1) };
                s.pending = false;
            }
            // SAFETY: the event was created in `iso_queue` and is closed once.
            unsafe { CloseHandle(s.event) };
        }
        // SAFETY: no transfer references the buffer any more.
        unsafe { WinUsb_UnregisterIsochBuffer(self.registration) };
    }
}
