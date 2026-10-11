//! Finding driver packages in the output of Windows' `pnputil /enum-drivers`.
#![cfg_attr(not(windows), allow(dead_code))]

/// Returns `(published name, original name)` for each of `wanted` that the
/// listing contains.
///
/// The listing describes each package in a block of `label: value` lines, the
/// published name (`oemNN.inf`) before the original name. Labels are
/// translated on non-English systems, so only the values are looked at.
pub fn find(listing: &str, wanted: &[&str]) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut published: Option<&str> = None;
    for line in listing.lines() {
        let Some(value) = line.split_whitespace().last() else {
            published = None;
            continue;
        };
        if is_published_name(value) {
            published = Some(value);
        } else if let Some(name) = wanted.iter().find(|w| w.eq_ignore_ascii_case(value)) {
            if let Some(oem) = published.take() {
                found.push((oem.to_string(), (*name).to_string()));
            }
        }
    }
    found
}

fn is_published_name(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower
        .strip_prefix("oem")
        .and_then(|rest| rest.strip_suffix(".inf"))
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = "\
Microsoft PnP Utility

Published Name:     oem6.inf
Original Name:      axefx2load.inf
Provider Name:      Fractal Audio Systems
Class Name:         USB

Published Name:     oem41.inf
Original Name:      fasusbaudio.inf
Provider Name:      Fractal Audio Systems

Veröffentlichter Name:     oem83.inf
Originalname:      FASUSBAUDIOKS.INF
Anbietername:      Fractal Audio Systems

Published Name:     oem126.inf
Original Name:      fireface_usb.inf
";

    #[test]
    fn finds_only_the_wanted_packages() {
        let found = find(LISTING, &["fasusbaudio.inf", "fasusbaudioks.inf"]);
        assert_eq!(
            found,
            vec![
                ("oem41.inf".to_string(), "fasusbaudio.inf".to_string()),
                ("oem83.inf".to_string(), "fasusbaudioks.inf".to_string()),
            ]
        );
    }

    #[test]
    fn the_loader_package_is_never_matched_by_the_audio_names() {
        assert!(find(LISTING, &["fasusbaudio.inf"]).iter().all(|(oem, _)| oem != "oem6.inf"));
    }

    #[test]
    fn tolerates_garbage() {
        assert!(find("", &["a.inf"]).is_empty());
        assert!(find("a.inf\n\n\noem.inf\noemx.inf", &["a.inf"]).is_empty());
    }
}
