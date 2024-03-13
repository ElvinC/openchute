use std::f64::consts::PI;

use eframe::egui;
use nalgebra::Vector2;
use super::geometry::{vec2, ToPoints};
use super::geometry;

use serde::{Serialize, Deserialize};

// Can be used to set options
pub trait ConfigurableGeometry: geometry::ToPoints {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame, use_imperial: bool, evaluator_context: &evalexpr::HashMapContext);
    fn update_from_context(&mut self, evaluator_context: &evalexpr::HashMapContext);
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
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
        ui.label("Line geometry:");

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

        self.update_from_context(evaluator_context);
    }

    fn update_from_context(&mut self, evaluator_context: &evalexpr::HashMapContext) {
        let eval = |expr: &str| evalexpr::eval_number_with_context(expr, evaluator_context).unwrap_or(0.0);

        self.line.begin.x = eval(&self.expressions[0]);
        self.line.begin.y = eval(&self.expressions[1]);
        self.line.end.x = eval(&self.expressions[2]);
        self.line.end.y = eval(&self.expressions[3]);
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
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
        ui.label("Ellipse geometry:");

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

        self.update_from_context(evaluator_context);
    }

    fn update_from_context(&mut self, evaluator_context: &evalexpr::HashMapContext) {
        let eval = |expr: &str| evalexpr::eval_number_with_context(expr, evaluator_context).unwrap_or(0.0);
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
            stop_angle: "0.5 * pi".into(),
            rotation: "0.0".into(),
            radius_x: "1.0".into(),
            radius_y: "1.0".into(), 
            center: ["0.0".into(), "0.0".into()] }
    }
}


#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigurablePointList {
    geo: geometry::Points,
    point_list: String,
    scale_x: String,
    scale_y: String,
    offset: [String; 2],
    scale_x_f: f64,
    scale_y_f: f64,
    offset_f: (f64, f64),
}

impl ToPoints for ConfigurablePointList {
    fn to_points(&self, resolution: u32) -> geometry::Points {
        self.geo.clone()
    }
}

impl ConfigurableGeometry for ConfigurablePointList {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame, use_imperial: bool, evaluator_context: &evalexpr::HashMapContext) {
        ui.label("Point list");

        ui.label("Scale x-y");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.scale_x).clip_text(false).desired_width(200.0));
            ui.add(egui::TextEdit::singleline(&mut self.scale_y).clip_text(false).desired_width(200.0));
        });

        ui.label("Offset x-y:");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.offset[0]).clip_text(false).desired_width(200.0));
            ui.add(egui::TextEdit::singleline(&mut self.offset[1]).clip_text(false).desired_width(200.0));
        });

        ui.label("Points (x,y):");
        ui.text_edit_multiline(&mut self.point_list);

        self.update_from_context(evaluator_context);
    }

    fn update_from_context(&mut self, evaluator_context: &evalexpr::HashMapContext) {
        let eval = |expr: &str| evalexpr::eval_number_with_context(expr, evaluator_context).unwrap_or(0.0);
        self.scale_x_f = eval(&self.scale_x);
        self.scale_y_f = eval(&self.scale_y);
        self.offset_f.0 = eval(&self.offset[0]);
        self.offset_f.1 = eval(&self.offset[1]);

        self.geo.points.clear();
        for pt in self.point_list.split("\n") {
            let coords: Vec<_> = pt.split(",").collect();
            if coords.len() != 2 {
                continue;
            }

            if let Some(x_coord) = coords.get(0) {
                if let Ok(x_val) = x_coord.trim().parse::<f64>() {
                    if let Some(y_coord) = coords.get(1) {
                        if let Ok(y_val) = y_coord.trim().parse::<f64>() {
                            self.geo.points.push(Vector2::new(x_val * self.scale_x_f + self.offset_f.0,
                                                              y_val * self.scale_y_f + self.offset_f.1))
                        }
                    }
                }
            }
        }
    }
}

impl ConfigurablePointList {
    pub fn new() -> Self {
        Self { geo: geometry::Points::new(), point_list: "0,0\n1,0".into(), scale_x: "1".into(), scale_y: "1".into(), offset: ["0.0".into(), "0.0".into()],
                offset_f: (0.0, 0.0), scale_x_f: 1.0, scale_y_f: 1.0, }   
    }
}