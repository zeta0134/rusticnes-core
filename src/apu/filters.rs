pub trait DspFilter {
    fn consume(&mut self, sample: f64);
    fn output(&self) -> f64;
}

pub struct HighPassIIR {
    alpha: f64,
    previous_output: f64,
    previous_input: f64,
    delta: f64,
}

impl HighPassIIR {
    pub fn new(sample_rate: f64, cutoff_frequency: f64) -> HighPassIIR {
        let delta_t = 1.0 / sample_rate;
        let time_constant = 1.0 / cutoff_frequency;
        let alpha = time_constant / (time_constant + delta_t);
        return HighPassIIR {
            alpha: alpha,
            previous_output: 0.0,
            previous_input: 0.0,
            delta: 0.0,
        }
    }
}

impl DspFilter for HighPassIIR {
    fn consume(&mut self, new_input: f64) {
        self.previous_output = self.output();
        self.delta = new_input - self.previous_input;
        self.previous_input = new_input;
    }

    fn output(&self) -> f64 {
        return self.alpha * self.previous_output + self.alpha * self.delta;
    }
}

pub struct LowPassIIR {
    alpha: f64,
    previous_output: f64,
    delta: f64,
}

impl LowPassIIR {
    pub fn new(sample_rate: f64, cutoff_frequency: f64) -> LowPassIIR {
        let delta_t = 1.0 / sample_rate;
        let time_constant = 1.0 / cutoff_frequency;
        let alpha = delta_t / (time_constant + delta_t);
        return LowPassIIR {
            alpha: alpha,
            previous_output: 0.0,
            delta: 0.0,
        }
    }
}

impl DspFilter for LowPassIIR {
    fn consume(&mut self, new_input: f64) {
        self.previous_output = self.output();
        self.delta = new_input - self.previous_output;
    }

    fn output(&self) -> f64 {
        return self.previous_output + self.alpha * self.delta;
    }
}
