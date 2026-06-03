use libwebrtc::ice_candidate::IceCandidate as LkIceCandidate;

use gkit_media::protocols::rtc::peer::core::IceCandidate;

// ---------------------------------------------------------------------------
// LkIceCandidate → our IceCandidate (infallible)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// parsed IceCandidate string → LkIceCandidate
// ---------------------------------------------------------------------------
pub fn lk_ice_from_parts(
    candidate: &str,
    sdp_mid: &str,
) -> Result<LkIceCandidate, String> {
    if candidate.is_empty() {
        return Err("empty candidate string".into());
    }
    // mline_index defaults to 0; our trait doesn't pass it explicitly
    LkIceCandidate::parse(sdp_mid, 0, candidate).map_err(|e| format!("ice parse error: {}", e))
}

#[cfg(test)]
mod tests {
    use crate::adapt::*;

    #[test]
    fn lk_ice_to_core() {
        let lk = LkIceCandidate::parse("0", 0, "candidate:0 1 UDP 2122252543 192.168.1.1 12345 typ host")
            .expect("parse");
        let ours: IceCandidate = crate::adapt::convert::lk_ice_candidate_to_core(lk);
        assert_eq!(ours.sdp_mid.as_deref(), Some("0"));
        assert_eq!(ours.sdp_mline_index, Some(0));
        assert!(ours.candidate.contains("192.168.1.1"));
    }

    #[test]
    fn core_parts_to_lk() {
        let lk = lk_ice_from_parts(
            "candidate:0 1 UDP 2122252543 192.168.1.1 12345 typ host",
            "0",
        )
        .expect("convert");
        assert_eq!(lk.sdp_mid(), "0");
        assert!(lk.candidate().contains("192.168.1.1"));
    }

    #[test]
    fn empty_candidate_fails() {
        assert!(lk_ice_from_parts("", "0").is_err());
    }
}
