#![allow(unused)]
use std::fs::{self, File};
use std::io::prelude::*;
use std::io::LineWriter;

#[derive(Clone)]
struct AtmosphericCondition {
    pressure: f64,
    density: f64,
    temperature: f64,
    sound_speed: f64,
}


const LAYERS: [(f64, f64); 8] = [(0.0, -0.0065), // ground
                                (11000.0, -0.0065), // Troposphere
                                (20000.0, 0.0), // Tropopause
                                (32000.0, 0.001), // Stratosphere
                                (47000.0, 0.0028), // Stratosphere
                                (51000.0, 0.0), // Stratopause
                                (71000.0, -0.0028), // Mesosphere
                                (86000.0, -0.0020)];  // Mesosphere

fn get_atmosphere(height: f64) -> AtmosphericCondition {
	let g0 = 9.80665; // m/s^2
    let t0 = 288.15;
    let p0 = 101325.0;
	let r = 287.05; // J/kgK
	let mut t = t0; // K
	let mut p = p0; // Pa

    if height > LAYERS.last().unwrap().0 {
        return AtmosphericCondition {
            density: 0.0,
            pressure: 0.0,
            temperature: 186.946,
            sound_speed: (1.4 * r * 186.946f64).sqrt(),
        }
    }
    let h = height;

    for i in 1..LAYERS.len() {
        let layer = LAYERS[i];
        let alpha = layer.1;
        let mut delta_h = layer.0 - LAYERS[i-1].0;
        if h <= layer.0 {
            // final layer
            delta_h = h - LAYERS[i-1].0;
        }

        let last_t = t;
        t += alpha * delta_h;
        if alpha == 0.0 {
            p *= (-g0 / (r*t) * delta_h).exp();
        } else {
            p *= (t/last_t).powf(-g0/(alpha * r));
        }

        if h <= layer.0 {
            break
        }
    }

    // Speed of sound
    let a = (1.4 * r * t).sqrt();
    let density = p / (r * t);

    AtmosphericCondition {
        density: density,
        pressure: p,
        temperature: t,
        sound_speed: a,
    }
}

#[derive(Clone)]
struct SimData {
    time: f64,
    altitude: f64,
    velocity: f64,
    acceleration: f64,
    force: f64,
    atmosphere: AtmosphericCondition,
    dynamic_pressure: f64,
    mach: f64,
    stagnation_temp: f64,
}

struct Sim {
    opening_time: f64, // Opening time in seconds
    mass: f64, // Mass in kg
    initial_altitude: f64, // Initial altitude in m
    initial_speed: f64, // Initial (downward) speed
    sim_results: Vec<SimData>,
    sim_updated: bool, //simulation results are up to date 

    area: f64,
    cd: f64,
}

impl Sim {
    fn new(mass: f64, altitude: f64, speed: f64, area: f64, cd: f64) -> Self {
        Self {
            opening_time: 0.0,
            mass: mass,
            initial_altitude: altitude,
            initial_speed: speed,
            sim_results: vec![],
            sim_updated: false,
            area: area,
            cd: cd,
        }
    }

    fn simulate(&mut self) -> Vec<SimData> {
        if self.sim_updated {
            return self.sim_results.clone();
        }
        
        let mut dt = 0.1; // initial timestep. Change to 1s when acceleration is lower
        let mut altitude = self.initial_altitude;
        let mut velocity = self.initial_speed; // Positive upwards
        let mut acceleration_old = -9.80665;
        let mut acceleration = -9.80665;
        let mut time = 0.0;

        for _ in 0..10000 {
            altitude += velocity * dt + 0.5 * acceleration * dt * dt;
            
            acceleration_old = acceleration;
            let atmo = get_atmosphere(altitude);
            let force = -0.5 * atmo.density * velocity * velocity * self.area * self.cd * velocity.signum();
            acceleration = -9.80665 + force / self.mass;
            // Velocity verlet
            velocity += 0.5 * (acceleration + acceleration_old) * dt;

            if altitude < 0.0 {
                break
            }

            dt = if acceleration < 1.0 { 0.5 } else { 0.1 };
            let mach = velocity / atmo.sound_speed;
            let stagnation = (1.0 + (1.4 - 1.0) / 2.0 * mach.powi(2)) * atmo.temperature;
            self.sim_results.push(SimData {
                acceleration: acceleration,
                altitude: altitude,
                force: force,
                time: time,
                velocity: velocity,
                dynamic_pressure: 0.5 * atmo.density * velocity * velocity,
                atmosphere: atmo,
                mach: mach,
                stagnation_temp: stagnation,
            });

            time += dt;
        }

        self.sim_updated = true;
        return self.sim_results.clone();
    }

    fn plot_altitude(&self) {
        for dat in self.sim_results.iter() {
            print!("[{:.2},{:.2},{:.2}, {:.1}],", dat.time, dat.altitude, dat.velocity, dat.dynamic_pressure);
        }
    }

    fn save_data(&self, filename: &str) {
        let file = File::create(filename).unwrap();
        let mut file = LineWriter::new(file);
        file.write_all(b"time[s],altitude[m],velocity[m/s],dynamic_pressure[Pa],force[N],stagnationT[K]\n").unwrap();

        for dat in self.sim_results.iter() {
            write!(file, "{:.2},{:.2},{:.2},{:.1},{:.1},{:.1}\n", dat.time, dat.altitude, dat.velocity, dat.dynamic_pressure, dat.force, dat.stagnation_temp);
        }

        file.flush().unwrap();
    }
}


#[cfg(test)]
mod tests {

    use super::Sim;

/*
    #[test]
    fn test_isa() {
        println!("DENSITY: {}", get_density(18000.0));
    }
 */
    #[test]
    fn test_sim() {
        let mut s = Sim::new(12.7, 120000.0, 0.0, 0.07, 1.0);
        s.simulate();
        s.save_data("data2.csv");
        println!("Finished simulation");
    }

}