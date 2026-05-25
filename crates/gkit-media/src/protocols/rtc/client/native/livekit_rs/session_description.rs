use std::str::FromStr;

use libwebrtc::session_description::{SdpType as LkSdpType, SessionDescription as LkSdp};

use crate::protocols::rtc::client::core::SessionDescription;

// ---------------------------------------------------------------------------
// LkSdp → our SessionDescription (infallible — just accessor calls)
// ---------------------------------------------------------------------------
impl From<LkSdp> for SessionDescription {
    fn from(sd: LkSdp) -> Self {
        let sdp_type = sd.sdp_type().to_string();
        let sdp = sd.to_string();
        SessionDescription { sdp_type, sdp }
    }
}

// ---------------------------------------------------------------------------
// our SessionDescription → LkSdp (fallible — requires SDP parsing)
// ---------------------------------------------------------------------------
pub fn lk_sdp_from_core(sd: &SessionDescription) -> Result<LkSdp, String> {
    if sd.sdp.is_empty() {
        return Err("empty SDP not parseable".into());
    }
    let sdp_type = LkSdpType::from_str(&sd.sdp_type)
        .map_err(|e| format!("invalid sdp_type '{}': {}", sd.sdp_type, e))?;
    LkSdp::parse(&sd.sdp, sdp_type).map_err(|e| format!("sdp parse error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn min_sdp() -> &'static str {
        "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n"
    }

    #[test]
    fn lk_sdp_to_core_offer() {
        let sdp = min_sdp();
        let lk = LkSdp::parse(sdp, LkSdpType::Offer).expect("parse");
        let ours: SessionDescription = lk.into();
        assert_eq!(ours.sdp_type, "offer");
        assert!(ours.sdp.contains("v=0"));
    }

    #[test]
    fn lk_sdp_to_core_answer() {
        let sdp = min_sdp();
        let lk = LkSdp::parse(sdp, LkSdpType::Answer).expect("parse");
        let ours: SessionDescription = lk.into();
        assert_eq!(ours.sdp_type, "answer");
    }

    #[test]
    fn core_to_lk_roundtrip() {
        let sdp = min_sdp();
        let ours = SessionDescription {
            sdp_type: "offer".into(),
            sdp: sdp.to_string(),
        };
        let lk = lk_sdp_from_core(&ours).expect("convert");
        let back: SessionDescription = lk.into();
        assert_eq!(back.sdp_type, "offer");
        assert!(back.sdp.contains("v=0"));
    }

    #[test]
    fn core_to_lk_empty_sdp_fails() {
        let ours = SessionDescription {
            sdp_type: "offer".into(),
            sdp: String::new(),
        };
        assert!(lk_sdp_from_core(&ours).is_err());
    }

    #[test]
    fn core_to_lk_invalid_type_fails() {
        let ours = SessionDescription {
            sdp_type: "bogus".into(),
            sdp: min_sdp().to_string(),
        };
        assert!(lk_sdp_from_core(&ours).is_err());
    }
}
