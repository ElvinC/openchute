use std::f64::consts::PI;

use eframe::egui;
use nalgebra::Vector2;
use super::geometry::{vec2, ToPoints};
use super::geometry;

// Can be used to set options
pub trait ConfigurableGeometry: geometry::ToPoints {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame, use_imperial: bool, evaluator_context: &evalexpr::HashMapContext);
}

#[derive(Clone)]
pub struct ConfigurableLine {
    line: geometry::Line,
    expressions: [String; 4],
}

impl ConfigurableLine {
    pub fn new() -> Self {
        Self {
            line: geometry::Line { begin: vec2(0.0, 0.0), end: vec2(1.0, 0.0)},
            expressions: ["0".to_string(), "0".to_string(), "1.0".to_string(), "0.0".to_string()].into()
        }
    }
}

impl ToPoints for ConfigurableLine {
    fn to_points(&self, resolution: u32) -> geometry::Points {
        self.line.to_points(resolution)
    }
}

impl ConfigurableGeometry for ConfigurableLine {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame, use_imperial: bool, evaluator_context: &evalexpr::HashMapContext) {
        let eval = |expr: &str| evalexpr::eval_number_with_context(expr, evaluator_context).unwrap_or(0.0);

        ui.label("Start point:");

        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.expressions[0]).clip_text(false).desired_width(200.0));
            ui.add(egui::TextEdit::singleline(&mut self.expressions[1]).clip_text(false).desired_width(200.0));
        });

        ui.label("End point:");

        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.expressions[2]).clip_text(false).desired_width(200.0));
            ui.add(egui::TextEdit::singleline(&mut self.expressions[3]).clip_text(false).desired_width(200.0));
        });

        self.line.begin.x = eval(&self.expressions[0]);
        self.line.begin.y = eval(&self.expressions[1]);
        self.line.end.x = eval(&self.expressions[2]);
        self.line.end.y = eval(&self.expressions[3]);
    }
}

#[derive(Clone)]
pub struct ConfigurableEllipse {
    ellipse: geometry::EllipseArc,
    start_angle: String,
    stop_angle: String,
    rotation: String, // CCW rotation of ellipse
    radius_x: String,
    radius_y: String,
    center: [String; 2],
}

impl ToPoints for ConfigurableEllipse {
    fn to_points(&self, resolution: u32) -> geometry::Points {
        self.ellipse.to_points(resolution)
    }
}

impl ConfigurableGeometry for ConfigurableEllipse {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame, use_imperial: bool, evaluator_context: &evalexpr::HashMapContext) {
        let eval = |expr: &str| evalexpr::eval_number_with_context(expr, evaluator_context).unwrap_or(0.0);

        ui.label("Angle start-stop:");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.start_angle).clip_text(false).desired_width(200.0));
            ui.add(egui::TextEdit::singleline(&mut self.stop_angle).clip_text(false).desired_width(200.0));
        });

        ui.label("Radius x-y");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.radius_x).clip_text(false).desired_width(200.0));
            ui.add(egui::TextEdit::singleline(&mut self.radius_y).clip_text(false).desired_width(200.0));
        });

        ui.label("Center:");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.center[0]).clip_text(false).desired_width(200.0));
            ui.add(egui::TextEdit::singleline(&mut self.center[1]).clip_text(false).desired_width(200.0));
        });

        ui.label("Rotation: ");
        ui.add(egui::TextEdit::singleline(&mut self.rotation).clip_text(false).desired_width(200.0));


        self.ellipse.start_angle = eval(&self.start_angle);
        self.ellipse.stop_angle = eval(&self.stop_angle);
        self.ellipse.rotation = eval(&self.rotation);
        self.ellipse.radius_x = eval(&self.radius_x);
        self.ellipse.radius_y = eval(&self.radius_y);
        self.ellipse.center[0] = eval(&self.center[0]);
        self.ellipse.center[1] = eval(&self.center[1]);
    }
}

impl ConfigurableEllipse {
    pub fn new() -> Self {
        Self {
            ellipse: geometry::EllipseArc::circle(1.0, vec2(0.0, 0.0)),
            start_angle: "0.0".into(),
            stop_angle: "2.0 * pi".into(),
            rotation: "0.0".into(),
            radius_x: "1.0".into(),
            radius_y: "1.0".into(), 
            center: ["0.0".into(), "0.0".into()] }
    }
}