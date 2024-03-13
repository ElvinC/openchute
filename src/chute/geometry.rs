#![allow(unused)]

// Geometrical operations

use std::ops::RangeInclusive;

use nalgebra::{Vector2, Rotation2};

use serde::{Serialize, Deserialize};

// Helper functions and values

const PI: f64 = std::f64::consts::PI;
const DEG2RAD: f64 = PI / 180.0;
const RAD2DEG: f64 = 180.0 / PI;


// Get the circumference of regular polygon with N sides compared to its circumscribed circle.
pub fn polygon_edge_len(n: u16) -> f64 {
	// Assume radius (center to corner) is 1
	let angle = 2.0 * PI / n as f64;
	let edge_length = (angle/2.0).sin() * 2.0;
	return edge_length as f64
}

pub fn polygon_to_circle_expansion(n: u16) -> f64 {
    let nf: f64 = n.max(1) as f64; // avoid zero division issues
    let angle = 2.0 * PI / nf as f64;
    let edge_length = (angle/2.0).sin() * 2.0;
    let ratio = 2.0 * PI / (edge_length * nf as f64);
    ratio
}

// Get the distance between the center and the side of a polygon, with a distance between the center and a corner of 1.
pub fn polygon_center_to_side(n: u16) -> f64 {
    let angle = 2.0 * PI / n as f64;
    (angle/2.0).cos()
}

pub fn vec2(x: f64, y: f64) -> Vector2<f64> {
    Vector2::new(x,y)
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Line {
    pub begin: Vector2<f64>,
    pub end: Vector2<f64>,
}

impl ToPoints for Line {
    fn to_points(&self, _resolution: u32) -> Points {
        Points::from_vec(vec![self.begin.clone(), self.end.clone()])
    }
}


#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct EllipseArc {
    pub start_angle: f64,
    pub stop_angle: f64,
    pub rotation: f64, // Clockwise rotation of ellipse
    pub radius_x: f64,
    pub radius_y: f64,
    pub center: Vector2<f64>,
}

impl EllipseArc {
    pub fn circle(radius: f64, center: Vector2<f64>) -> Self {
        Self {
            start_angle: 0.0,
            stop_angle: 2.0 * PI,
            rotation: 0.0, 
            radius_x: radius,
            radius_y: radius,
            center: center 
        }
    }
}

impl ToPoints for EllipseArc {
    fn to_points(&self, resolution: u32) -> Points {
        let start = self.start_angle % (2.0 * PI);
        let stop = if self.stop_angle > 2.0 * PI { self.stop_angle % (2.0 * PI) } else { self.stop_angle };
        let diff = stop - start;
        let num_steps = ((diff / (2.0 * PI) * (resolution as f64)) as usize).max(3);

        let mut result = vec![];

        for idx in 0..=num_steps {
            let angle = start + (idx as f64 / num_steps as f64) * diff;
            result.push(Vector2::new(angle.cos() * self.radius_x, angle.sin()*self.radius_y))
        }

        if self.rotation != 0.0 || self.center.x != 0.0 || self.center.y != 0.0 {
            let rot = Rotation2::new(self.rotation);
            result = result.iter().map(|pt| rot.transform_vector(pt) + self.center).collect();
        }

        Points::from_vec(result)
    }
}

// A generic bezier curve
#[derive(Clone, Serialize, Deserialize)]
struct BezierSpline {
    control_points: Vec<(Vector2<f64>, Vector2<f64>, Vector2<f64>)>, // Handle1, center, handle2
}

impl BezierSpline {
    fn new() -> Self {
        Self { control_points: vec![] }
    }

    fn add_control(&mut self, pt1: Vector2<f64>, pt2: Vector2<f64>, pt3: Vector2<f64>) {
        self.control_points.push((pt1,pt2,pt3));
    }
}

impl ToPoints for BezierSpline {
    fn to_points(&self, resolution: u32) -> Points {
        let mut result = vec![];

        if self.control_points.len() == 0 {
            return Points::from_vec(result);
        }

        if self.control_points.len() == 1 {
            result.push(self.control_points.first().unwrap().1); // Add center point
            return Points::from_vec(result);
        }

        for idx in 0..(self.control_points.len()-1) {
            let pt0 = self.control_points[idx].1;
            let pt1 = self.control_points[idx].2;
            let pt2 = self.control_points[idx+1].0;
            let pt3 = self.control_points[idx+1].1;

            for sample_idx in 0..resolution {
                let t = sample_idx as f64 / resolution as f64; // Interpolation value
                // bernsteins polynomial
                let t2 = t*t;
                let t3 = t2*t;
                let mut new_point = pt0 * (1.0 - 3.0 * t + 3.0 * t2 - t3);
                new_point +=  pt1 * (3.0 * t - 6.0 * t2 + 3.0 * t3);
                new_point +=  pt2 * (3.0 * t2 - 3.0 * t3);
                new_point +=  pt3 * t3;
                result.push(new_point);
            }
        }

        // Add last point
        result.push(self.control_points.last().unwrap().1);

        Points::from_vec(result)
    }
}


// A collection of points
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Points {
    pub points: Vec<Vector2<f64>>,
}

impl Points {
    pub fn new() -> Self {
        Self { points: vec![] }
    }

    pub fn iter(&self) -> impl Iterator<Item=&Vector2<f64>> + '_ {
        self.points.iter()
    }

    pub fn from_vec(points: Vec<Vector2<f64>>) -> Self {
        Self { points: points }
    }
    
    pub fn bounds(&self) -> (Vector2<f64>, Vector2<f64>) {
        let mut min = self.points[0];
        let mut max = self.points[0];

        for pt in self.points.iter() {
            min.x = min.x.min(pt.x);
            min.y = min.y.min(pt.y);
            max.x = max.x.max(pt.x);
            max.y = max.y.max(pt.y);
        }

        (min, max)
    }

    pub fn mirror_x(&self) -> Self {
        // returns a copy mirrored around the X axis
        let mut new = self.clone();
        for point in new.points.iter_mut() {
            point.x = -point.x;
        }
        new
    }

    pub fn get_first_point(&self) -> Vector2<f64> {
        self.points.first().unwrap().clone()
    }

    pub fn get_last_point(&self) -> Vector2<f64> {
        self.points.last().unwrap().clone()
    }

}

pub trait ToPoints {
    // Resolution is roughly number of points in full circle
    // 200 default is fine
    fn to_points(&self, resolution: u32) -> Points;
}

struct GeometryOption {
    name: String,
    value: f64,
    range: RangeInclusive<f64>,
}






#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use nalgebra::Vector2;

    use super::{EllipseArc, ToPoints, BezierSpline, polygon_to_circle_expansion};

    #[test]
    fn test_ellipse() {
        let mut ell = EllipseArc {
            center: Vector2::new(0.0, 0.0),
            radius_x: 1.0,
            radius_y: 0.5,
            rotation: 45.0 * PI/180.0,
            start_angle: 0.0,
            stop_angle: 270.0 * PI/180.0,
        };

        for pt in ell.to_points(100).iter() {
            print!("[{:.4},{:.4}],", pt.x, pt.y);
        }

        let mut circ = EllipseArc::circle(4.0, Vector2::zeros());
        println!("Bounds: {:?}", circ.to_points(360).bounds());

    }

    #[test]
    fn test_bezier() {
        let mut bez = BezierSpline::new();
        bez.add_control(Vector2::new(0.0, 0.0), Vector2::new(0.0, 0.0), Vector2::new(1.0, 0.0));
        bez.add_control(Vector2::new(0.0, 1.0), Vector2::new(1.0, 1.0), Vector2::new(1.0, 1.0));
        println!("{:?}", bez.to_points(50).points);
    }

    #[test]
    fn test_polygon_to_circle() {
        let test = polygon_to_circle_expansion(2);
        println!("{}", test);
        assert_eq!(test, 2.0 * PI / (4.0));
    }
}
