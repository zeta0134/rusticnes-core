#![allow(non_snake_case)]

use std::f64::consts::PI;

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
        let time_constant = 1.0 / (2.0 * PI * cutoff_frequency);
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

fn blackman_window(index: usize, window_size: usize) -> f64 {
    let i = index as f64;
    let M = window_size as f64;
    return 0.42 - 0.5 * ((2.0 * PI * i) / M).cos() + 0.08 * ((4.0 * PI * i) / M).cos();
}

fn sinc(index: usize, window_size: usize, fc: f64) -> f64 {
    let i = index as f64;
    let M = window_size as f64;
    let shifted_index = i - (M / 2.0);
    if index == (window_size / 2) {
        return 2.0 * PI * fc;
    } else {
        return (2.0 * PI * fc * shifted_index).sin() / shifted_index;
    }
}

fn normalize(input: Vec<f64>) -> Vec<f64> {
    let sum: f64 = input.clone().into_iter().sum();
    let output = input.into_iter().map(|x| x / sum).collect();
    return output;
}

fn windowed_sinc_kernel(fc: f64, window_size: usize) -> Vec<f64> {
    let mut kernel: Vec<f64> = Vec::new();
    for i in 0 ..= window_size {
        kernel.push(sinc(i, window_size, fc) * blackman_window(i, window_size));
    }
    return normalize(kernel);
}

pub struct LowPassFIR {
    kernel: Vec<f64>,
    inputs: Vec<f64>,
    input_index: usize,
}

impl LowPassFIR {
    pub fn new(sample_rate: f64, cutoff_frequency: f64, window_size: usize) -> LowPassFIR {
        let fc = cutoff_frequency / sample_rate;
        let kernel = windowed_sinc_kernel(fc, window_size);
        let mut inputs = Vec::new();
        inputs.resize(window_size + 1, 0.0);

        // debug!
        println!("fc: {}", fc);
        println!("Low pass kernel");
        for i in 0 .. kernel.len() {
            println!("{}", kernel[i]);
        }

        return LowPassFIR {
            kernel: kernel,
            inputs: inputs,
            input_index: 0,
        }
    }
}

impl DspFilter for LowPassFIR {
    fn consume(&mut self, new_input: f64) {
        self.inputs[self.input_index] = new_input;
        self.input_index = (self.input_index + 1) % self.inputs.len();
    }

    fn output(&self) -> f64 {
        let mut output: f64 = 0.0;
        for i in 0 .. self.inputs.len() {
            let buffer_index = (self.input_index + i) % self.inputs.len();
            output += self.kernel[i] * self.inputs[buffer_index];
        }
        return output;
    }
}