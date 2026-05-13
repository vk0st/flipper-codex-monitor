use chrono::{Local, TimeZone};
use serde::{Deserialize, Serialize};

pub const STATUS_OK: u8 = 0;
pub const STATUS_STALE: u8 = 1;
pub const STATUS_CODEX_ERROR: u8 = 2;
pub const STATUS_LIMIT_REACHED: u8 = 3;

pub const CODEX_LIMITS_PACKET_SIZE: usize = 21;

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct CodexLimitsPacket {
    pub five_hour_used_percent: u8,
    pub five_hour_reset: [u8; 8],
    pub week_used_percent: u8,
    pub week_reset: [u8; 10],
    pub status: u8,
}

impl CodexLimitsPacket {
    pub fn error() -> Self {
        Self {
            five_hour_used_percent: 0,
            five_hour_reset: fixed_bytes("--:--"),
            week_used_percent: 0,
            week_reset: fixed_bytes("--- --:--"),
            status: STATUS_CODEX_ERROR,
        }
    }

    pub fn with_status(&self, status: u8) -> Self {
        let mut packet = self.clone();
        packet.status = status;
        packet
    }
}

#[derive(Deserialize)]
struct RateLimitsRpcResponse {
    result: RateLimitsReadResult,
}

#[derive(Deserialize)]
struct RateLimitsReadResult {
    #[serde(rename = "rateLimits")]
    rate_limits: RateLimitsSnapshot,
}

#[derive(Deserialize)]
struct RateLimitsSnapshot {
    primary: RateLimitWindow,
    secondary: RateLimitWindow,
    #[serde(default, rename = "rateLimitReachedType")]
    rate_limit_reached_type: Option<String>,
}

#[derive(Deserialize)]
struct RateLimitWindow {
    #[serde(rename = "usedPercent")]
    used_percent: i16,
    #[serde(rename = "resetsAt")]
    resets_at: i64,
}

pub fn packet_from_rate_limits_json(
    json: &str,
) -> Result<CodexLimitsPacket, Box<dyn std::error::Error + Send + Sync>> {
    let response: RateLimitsRpcResponse = serde_json::from_str(json)?;
    packet_from_rate_limits(response.result.rate_limits)
}

pub fn packet_from_rate_limits_result_value(
    result: serde_json::Value,
) -> Result<CodexLimitsPacket, Box<dyn std::error::Error + Send + Sync>> {
    let result: RateLimitsReadResult = serde_json::from_value(result)?;
    packet_from_rate_limits(result.rate_limits)
}

fn packet_from_rate_limits(
    limits: RateLimitsSnapshot,
) -> Result<CodexLimitsPacket, Box<dyn std::error::Error + Send + Sync>> {
    Ok(CodexLimitsPacket {
        five_hour_used_percent: clamp_percent(limits.primary.used_percent),
        five_hour_reset: fixed_bytes(&format_reset(limits.primary.resets_at, "%H:%M")?),
        week_used_percent: clamp_percent(limits.secondary.used_percent),
        week_reset: fixed_bytes(&format_reset(limits.secondary.resets_at, "%a %H:%M")?),
        status: if limits.rate_limit_reached_type.is_some() {
            STATUS_LIMIT_REACHED
        } else {
            STATUS_OK
        },
    })
}

fn clamp_percent(value: i16) -> u8 {
    value.clamp(0, 100) as u8
}

fn format_reset(
    timestamp: i64,
    format: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let Some(datetime) = Local.timestamp_opt(timestamp, 0).single() else {
        return Err(format!("invalid reset timestamp: {timestamp}").into());
    };

    Ok(datetime.format(format).to_string())
}

fn fixed_bytes<const N: usize>(value: &str) -> [u8; N] {
    let mut out = [0; N];
    let bytes = value.as_bytes();
    let copy_len = bytes.len().min(N.saturating_sub(1));
    out[..copy_len].copy_from_slice(&bytes[..copy_len]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    const RATE_LIMITS_RESPONSE: &str = r#"{
        "id": 7,
        "result": {
            "rateLimits": {
                "limitId": "codex",
                "limitName": null,
                "primary": {
                    "usedPercent": 21,
                    "windowDurationMins": 300,
                    "resetsAt": 1778698720
                },
                "secondary": {
                    "usedPercent": 47,
                    "windowDurationMins": 10080,
                    "resetsAt": 1778743867
                },
                "credits": {
                    "hasCredits": false,
                    "unlimited": false,
                    "balance": "0"
                },
                "planType": "prolite",
                "rateLimitReachedType": null
            },
            "rateLimitsByLimitId": {
                "codex": {
                    "limitId": "codex",
                    "primary": {
                        "usedPercent": 21,
                        "windowDurationMins": 300,
                        "resetsAt": 1778698720
                    },
                    "secondary": {
                        "usedPercent": 47,
                        "windowDurationMins": 10080,
                        "resetsAt": 1778743867
                    },
                    "rateLimitReachedType": null
                },
                "codex_bengalfox": {
                    "limitId": "codex_bengalfox",
                    "limitName": "GPT-5.3-Codex-Spark",
                    "primary": {
                        "usedPercent": 0,
                        "windowDurationMins": 300,
                        "resetsAt": 1778709110
                    },
                    "secondary": {
                        "usedPercent": 0,
                        "windowDurationMins": 10080,
                        "resetsAt": 1779295910
                    },
                    "rateLimitReachedType": null
                }
            }
        }
    }"#;

    #[test]
    fn parses_aggregate_limits_and_ignores_per_model_limits() {
        let packet = packet_from_rate_limits_json(RATE_LIMITS_RESPONSE).unwrap();

        assert_eq!(packet.five_hour_used_percent, 21);
        assert_eq!(packet.week_used_percent, 47);
        assert_eq!(packet.status, STATUS_OK);

        let expected_five_hour_reset = Local
            .timestamp_opt(1778698720, 0)
            .single()
            .unwrap()
            .format("%H:%M")
            .to_string();
        let expected_week_reset = Local
            .timestamp_opt(1778743867, 0)
            .single()
            .unwrap()
            .format("%a %H:%M")
            .to_string();

        assert_eq!(
            nul_trimmed(&packet.five_hour_reset),
            expected_five_hour_reset
        );
        assert_eq!(nul_trimmed(&packet.week_reset), expected_week_reset);
    }

    #[test]
    fn clamps_percentages_and_marks_reached_limits() {
        let response = RATE_LIMITS_RESPONSE
            .replace("\"usedPercent\": 21", "\"usedPercent\": 135")
            .replace("\"usedPercent\": 47", "\"usedPercent\": -7")
            .replace(
                "\"rateLimitReachedType\": null",
                "\"rateLimitReachedType\": \"primary\"",
            );

        let packet = packet_from_rate_limits_json(&response).unwrap();

        assert_eq!(packet.five_hour_used_percent, 100);
        assert_eq!(packet.week_used_percent, 0);
        assert_eq!(packet.status, STATUS_LIMIT_REACHED);
    }

    #[test]
    fn serializes_to_the_flipper_packet_size() {
        let packet = packet_from_rate_limits_json(RATE_LIMITS_RESPONSE).unwrap();
        let bytes = bincode::serialize(&packet).unwrap();

        assert_eq!(bytes.len(), CODEX_LIMITS_PACKET_SIZE);
    }

    fn nul_trimmed(bytes: &[u8]) -> String {
        let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
        String::from_utf8(bytes[..end].to_vec()).unwrap()
    }
}
