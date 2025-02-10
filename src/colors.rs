pub struct RgbaColor(f64, f64, f64, f64);
impl RgbaColor {
    pub fn new<R, G, B, A>((r, g, b, a): (R, G, B, A)) -> Option<Self>
    where
        R: Into<f64>,
        G: Into<f64>,
        B: Into<f64>,
        A: Into<f64>,
    {
        // Shadowing
        let r = r.into();
        let g = g.into();
        let b = b.into();
        let a = a.into();

        // Confirm
        let is_invalid = |x: &f64| {
            if 0.0 > *x || *x > 1.0 {
                false
            } else {
                true
            }
        };

        // I <3 Functional Programming
        let proceed = vec![r, g, b, a].iter().any(|x| is_invalid(x));

        // Return
        if proceed {
            Some(RgbaColor(r.into(), g.into(), b.into(), a.into()))
        } else {
            None
        }
    }
}

impl RgbaColor {
    pub fn red(&self) -> f64 {
        self.0
    }

    pub fn green(&self) -> f64 {
        self.1
    }

    pub fn blue(&self) -> f64 {
        self.2
    }

    pub fn alpha(&self) -> f64 {
        self.3
    }
}
