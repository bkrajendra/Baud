use chrono::{DateTime, Local};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    None,
    Lf,
    Cr,
    CrLf,
}

impl LineEnding {
    pub const ALL: [LineEnding; 4] = [
        LineEnding::None,
        LineEnding::Lf,
        LineEnding::Cr,
        LineEnding::CrLf,
    ];

    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            LineEnding::None => b"",
            LineEnding::Lf => b"\n",
            LineEnding::Cr => b"\r",
            LineEnding::CrLf => b"\r\n",
        }
    }
}

impl std::fmt::Display for LineEnding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            LineEnding::None => "None",
            LineEnding::Lf => "\\n",
            LineEnding::Cr => "\\r",
            LineEnding::CrLf => "\\r\\n",
        };
        write!(f, "{label}")
    }
}

pub fn format_timestamp(now: DateTime<Local>) -> String {
    now.format("%H:%M:%S%.3f").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn line_ending_bytes() {
        assert_eq!(LineEnding::None.as_bytes(), b"");
        assert_eq!(LineEnding::Lf.as_bytes(), b"\n");
        assert_eq!(LineEnding::Cr.as_bytes(), b"\r");
        assert_eq!(LineEnding::CrLf.as_bytes(), b"\r\n");
    }

    #[test]
    fn line_ending_display_labels() {
        assert_eq!(LineEnding::None.to_string(), "None");
        assert_eq!(LineEnding::Lf.to_string(), "\\n");
        assert_eq!(LineEnding::Cr.to_string(), "\\r");
        assert_eq!(LineEnding::CrLf.to_string(), "\\r\\n");
    }

    #[test]
    fn timestamp_formats_hh_mm_ss_millis() {
        let dt = chrono::Local
            .with_ymd_and_hms(2026, 8, 9, 14, 3, 22)
            .unwrap()
            + chrono::Duration::milliseconds(451);
        assert_eq!(format_timestamp(dt), "14:03:22.451");
    }
}
