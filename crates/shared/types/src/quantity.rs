//! Kubernetes quantities: `250m` of a core, `1Gi` of memory, read into base units and written back.

const SUFFIXES: [(&str, f64); 15] = [
    ("Ki", 1024.0),
    ("Mi", 1_048_576.0),
    ("Gi", 1_073_741_824.0),
    ("Ti", 1_099_511_627_776.0),
    ("Pi", 1_125_899_906_842_624.0),
    ("Ei", 1_152_921_504_606_846_976.0),
    ("n", 1e-9),
    ("u", 1e-6),
    ("m", 1e-3),
    ("k", 1e3),
    ("M", 1e6),
    ("G", 1e9),
    ("T", 1e12),
    ("P", 1e15),
    ("E", 1e18),
];

/// `250m` reads 0.25, `1Gi` reads 1073741824, `2` reads 2.
pub fn quantity(text: &str) -> Option<f64> {
    let text = text.trim();
    for (suffix, factor) in SUFFIXES {
        if let Some(number) = text.strip_suffix(suffix)
            && let Ok(value) = number.parse::<f64>()
        {
            return Some(value * factor);
        }
    }
    text.parse().ok()
}

/// Cores as `kubectl top` writes them: `180m`, `2`.
pub fn cores(value: f64) -> String {
    let milli = (value * 1000.0).round();
    match milli % 1000.0 == 0.0 && milli > 0.0 {
        true => format!("{}", milli / 1000.0),
        false => format!("{milli}m"),
    }
}

/// Bytes in the largest binary unit that keeps a whole number: `700Mi`, `2Gi`.
pub fn bytes(value: f64) -> String {
    let units = [("Gi", 1_073_741_824.0), ("Mi", 1_048_576.0), ("Ki", 1024.0)];
    for (unit, size) in units {
        if value >= size {
            return format!("{}{unit}", (value / size).round());
        }
    }
    format!("{}", value.round())
}

/// One resource's column: where the request and the usage stand, as fractions of its height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gauge {
    pub request: Option<f64>,
    pub used: Option<f64>,
    pub pressure: Pressure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pressure {
    Under,
    /// Past the request.
    Over,
    /// Within a tenth of the limit.
    Near,
}

/// The column's top is the limit, or a quarter above the larger of request and usage.
pub fn gauge(request: Option<f64>, limit: Option<f64>, used: Option<f64>) -> Option<Gauge> {
    let larger = request.into_iter().chain(used).fold(0.0, f64::max);
    let top = limit.unwrap_or(larger * 1.25);
    if top <= 0.0 {
        return None;
    }
    let part = |value: f64| (value / top).clamp(0.0, 1.0);
    let pressure = match (used, request, limit) {
        (Some(used), _, Some(limit)) if used >= limit * 0.9 => Pressure::Near,
        (Some(used), Some(request), _) if used > request => Pressure::Over,
        _ => Pressure::Under,
    };
    Some(Gauge {
        request: request.map(part),
        used: used.map(part),
        pressure,
    })
}
