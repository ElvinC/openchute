#![allow(unused)]



extern crate nalgebra as na;
use std::default;

use dxf;
use dxf::Drawing;
use dxf::entities::*;
use eframe::egui;
use egui_plot;
use evalexpr::ContextWithMutableVariables;
use uom::unit;
use crate::chute::parachute;
use crate::chute::ui;
use crate::chute::geometry;

use nalgebra::Vector2;

use std::f64::consts::PI;

use uom::si::{self, length};

use super::configurable_shapes::ConfigurableGeometry;
use super::geometry::ToPoints;
use super::configurable_shapes;
use geometry::vec2;

use serde::{Serialize, Deserialize};
use serde_json;


#[derive(Clone, Serialize, Deserialize)]
pub enum GeometryType {
    Line(configurable_shapes::ConfigurableLine),
    EllipseArc(configurable_shapes::ConfigurableEllipse),
}

impl geometry::ToPoints for GeometryType {
    fn to_points(&self, resolution: u32) -> geometry::Points {
        match self {
            Self::Line(config) => config.to_points(resolution),
            Self::EllipseArc(config) => config.to_points(resolution)
        }
    }
}

impl GeometryType {
    fn update_from_context(&mut self, evaluator_context: &evalexpr::HashMapContext) {
        match self {
            Self::Line(config) => config.update_from_context(evaluator_context),
            Self::EllipseArc(config) => config.update_from_context(evaluator_context)
        }
    }
}

// Represents a band. Can contain multiple geometries representing the cross section, but is symmetrical and has a fixed number of gores
#[derive(Clone, Serialize, Deserialize)]
pub struct PolygonalChuteSection {
    objects: Vec<GeometryType>,

}

impl PolygonalChuteSection {
    fn add_line(&mut self) {
        self.objects.push(GeometryType::Line(
            configurable_shapes::ConfigurableLine::new(),
        ));
    }

    fn add_ellipse(&mut self) {
        self.objects.push(GeometryType::EllipseArc(
            configurable_shapes::ConfigurableEllipse::new(),
        ));
    }

    fn update_from_context(&mut self, evaluator_context: &evalexpr::HashMapContext) {
        for object in self.objects.iter_mut() {
            object.update_from_context(evaluator_context);
        }
    }
}

impl Default for PolygonalChuteSection {
    fn default() -> Self {
        Self { objects: vec![] }
    }
}

// Represents a chute section that can be bent into a perfectly circular disk/cone/cylinder. Only a straight line allowed
#[derive(Clone, Serialize, Deserialize)]
pub struct CircularChuteSection {
    line: geometry::Line,
    expressions: [String; 4]
}

impl Default for CircularChuteSection {
    fn default() -> Self {
        Self {
            line: geometry::Line { begin: Vector2::new(0.0, 0.0), end: Vector2::new(1.0, 0.0)},
            expressions: ["0".to_string(), "0".to_string(), "1.0".to_string(), "0.0".to_string()].into()
        }
    }
}


#[derive(Clone, Serialize, Deserialize)]
pub enum ChuteSectionType {
    Polygonal(PolygonalChuteSection),
    Circular(CircularChuteSection)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChuteSection {
    section_type: ChuteSectionType,
    gores: u16,
    fabric: FabricSelector,
    seam_allowance: (f64, f64, f64, f64), // Right, top, left, bottom
    corner_cutout: bool,
    colors: Vec<[f32; 3]>, // Colors. If less than number of gores, it continues repeating
}

impl ChuteSection {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, use_imperial: bool, evaluator_context: &evalexpr::HashMapContext, index_id: u16) {
        ui.label("Fabric:");
        self.fabric.ui(ui, frame, use_imperial, index_id);
        
        // Seam allowance input
        ui.label("Seam allowance (right, left)");
        ui.horizontal(|ui| {
            ui::length_slider(ui, &mut self.seam_allowance.0, use_imperial, 0.0..=0.1, &length::millimeter, &length::inch);
            ui::length_slider(ui, &mut self.seam_allowance.2, use_imperial, 0.0..=0.1, &length::millimeter, &length::inch);
        });
        ui.label("Seam allowance (top, bottom)");
        ui.horizontal(|ui| {
            ui::length_slider(ui, &mut self.seam_allowance.1, use_imperial, 0.0..=0.1, &length::millimeter, &length::inch);
            ui::length_slider(ui, &mut self.seam_allowance.3, use_imperial, 0.0..=0.1, &length::millimeter, &length::inch);
        });

        ui.checkbox(&mut self.corner_cutout, "Cut out seam allowance corners");

        ui.label("Number of gores:").on_hover_text("Number of parachute gores. Typically between 6 and 24");
        ui::integer_edit_field(ui, &mut self.gores);

        ui.horizontal(|ui| {
            ui.label("Color:");
            if ui.button("➕").clicked() {
                // Alternate between international orange and black
                if self.colors.len() % 2 == 0 {
                    self.colors.push([1.0, 0.31, 0.0]);
                } else {
                    self.colors.push([0.0, 0.0, 0.0]);
                }
            };

            if ui.add_enabled(self.colors.len() > 0, egui::Button::new("➖")).clicked() {
                self.colors.pop().unwrap_or_default();
            }

            for color in self.colors.iter_mut() {
                ui.color_edit_button_rgb(color);
            }
        });

        ui.separator();

        let eval = |expr: &str| evalexpr::eval_number_with_context(expr, evaluator_context).unwrap_or(0.0);
        
        match &mut self.section_type {
            ChuteSectionType::Circular(sec) => {
                ui.label("Start point:");

                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut sec.expressions[0]).clip_text(false).desired_width(200.0));
                    ui.add(egui::TextEdit::singleline(&mut sec.expressions[1]).clip_text(false).desired_width(200.0));
                });

                ui.label("End point:");

                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut sec.expressions[2]).clip_text(false).desired_width(200.0));
                    ui.add(egui::TextEdit::singleline(&mut sec.expressions[3]).clip_text(false).desired_width(200.0));
                });

                sec.line.begin.x = eval(&sec.expressions[0]);
                sec.line.begin.y = eval(&sec.expressions[1]);
                sec.line.end.x = eval(&sec.expressions[2]);
                sec.line.end.y = eval(&sec.expressions[3]);

            },
            ChuteSectionType::Polygonal(sec) => {
                ui.horizontal(|ui| {
                    if ui.button("Add line").clicked() {
                        sec.add_line();
                    }

                    if ui.button("Add ellipse arc").clicked() {
                        sec.add_ellipse();
                    }
                });

                ui.separator();

                let mut to_delete: Option<usize> = None;

                for (idx, object) in sec.objects.iter_mut().enumerate() {
                    if ui.button("❌").clicked() {
                        to_delete = Some(idx);
                    }
                    match object {
                        GeometryType::Line(config) => config.ui(ui, frame, use_imperial, evaluator_context),
                        GeometryType::EllipseArc(config) => config.ui(ui, frame, use_imperial, evaluator_context),
                    }
                }

                if let Some(delete_idx) = to_delete {
                    sec.objects.remove(delete_idx);
                }

            }
        }
    }

    pub fn update_from_context(&mut self, evaluator_context: &evalexpr::HashMapContext) {
        let eval = |expr: &str| evalexpr::eval_number_with_context(expr, evaluator_context).unwrap_or(0.0);
        match &mut self.section_type {
            ChuteSectionType::Circular(sec) => {
                sec.line.begin.x = eval(&sec.expressions[0]);
                sec.line.begin.y = eval(&sec.expressions[1]);
                sec.line.end.x = eval(&sec.expressions[2]);
                sec.line.end.y = eval(&sec.expressions[3]);
            },
            ChuteSectionType::Polygonal(sec) => {
                sec.update_from_context(evaluator_context);
            }
        }
    }

    fn new_circular() -> Self {
        Self {
            section_type: ChuteSectionType::Circular(CircularChuteSection::default()),
            gores: 8,
            fabric: FabricSelector::new(),
            seam_allowance: (0.01, 0.01, 0.01, 0.01),
            colors: vec![[1.0, 0.31, 0.0], [0.0, 0.0, 0.0]],
            corner_cutout: false,
        }
    }

    fn new_polygonal() -> Self {
        Self {
            section_type: ChuteSectionType::Polygonal(PolygonalChuteSection::default()),
            gores: 8,
            fabric: FabricSelector::new(),
            seam_allowance: (0.01, 0.01, 0.01, 0.01),
            colors: vec![[1.0, 0.31, 0.0], [0.0, 0.0, 0.0]],
            corner_cutout: false,
        }
    }

    fn get_cross_section(&self, resolution: u32, expanded_polygon: bool) -> geometry::Points {
        // expanded_polygon expands the polygonal geometry so the circumference is equal the nominal circular circumference.
        // Not used in 2D plot, only when generating gores or in 3D plot (corner locations)

        // to 2D section
        match &self.section_type {
            ChuteSectionType::Circular(circ) => {
                circ.line.to_points(resolution)
            },
            ChuteSectionType::Polygonal(poly) => {
                let mut pts = geometry::Points::new();

                for object in &poly.objects {
                    if expanded_polygon {
                        let mut cross_section = object.to_points(resolution).points;
                        let expansion_ratio = geometry::polygon_to_circle_expansion(self.gores);
                        cross_section.iter_mut().for_each(|pt| {
                            pt.x *= expansion_ratio;
                        });

                        pts.points.append(&mut cross_section);
                    } else {
                        pts.points.append(&mut object.to_points(resolution).points);
                    }
                }

                pts
            }
        }
    }

    fn to_pattern_piece(&self, resolution: u32) -> PatternPiece {
        match &self.section_type {
            ChuteSectionType::Circular(circ) => {
                // Generate flat piece that can be folded into cone section/cylinder/disk
                let mut piece = PatternPiece::new();
                piece.set_corner_cutout(self.corner_cutout);
                let pt1 = circ.line.begin;
                let pt2 = circ.line.end;

                let (pt1, pt2, allowance) = if circ.line.end.x >= circ.line.begin.x {
                    (circ.line.begin, circ.line.end, self.seam_allowance)
                } else {
                    // Rotate seam allowance 180 deg
                    (circ.line.end, circ.line.begin, (self.seam_allowance.2, self.seam_allowance.3, self.seam_allowance.0, self.seam_allowance.1))
                };
                // PT2 is "outer" one

                let distance = (pt1 - pt2).norm(); // band size of the half circle
                if distance < 1e-6 || self.gores == 0 {
                    // Empty pattern piece
                    return piece;
                } else if (pt2.x - pt1.x).abs() < 1e-6 {
                    // Vertical band. Just generate a rectangle. Bottom of rectangle = towards parachute skirt
                    // Center horizontally.
                    // Start vertically at y=0
                    let pos_x = pt1.x * 2.0 * PI/(self.gores as f64)/2.0; // x coord
                    let pos_y = (pt1.y - pt2.y).abs(); // y coord

                    let bottom_left = vec2(-pos_x, 0.0);
                    let bottom_right = vec2(pos_x, 0.0);
                    let top_left = vec2(-pos_x, pos_y);
                    let top_right = vec2(pos_x, pos_y);

                    // Define segments clockwise to make seams work correctly

                    // Right segment
                    piece.add_segment(Segment::from_vec(vec![bottom_right, top_right], allowance.0));
                    // Top segment
                    piece.add_segment(Segment::from_vec(vec![top_right, top_left], allowance.1));
                    // Left segment
                    piece.add_segment(Segment::from_vec(vec![top_left, bottom_left], allowance.2));
                    // Bottom segment
                    piece.add_segment(Segment::from_vec(vec![bottom_left, bottom_right], allowance.3));

                    return piece;                    
                }

                
                let y_intersect = -pt1.x * ((pt2.y - pt1.y)/(pt2.x - pt1.x));
                let inner_radius = (pt1.x.powi(2) + y_intersect.powi(2)).sqrt();
                let outer_radius = distance + inner_radius;

                let angle = (pt2.x)/(outer_radius) * 2.0 * PI / self.gores as f64;


                // Different cases. If top seam allowance is greater than top radius, or inner_radius is zero:
                if angle >= 1.999 * PI && inner_radius <= allowance.1 {
                    // Make full circle, no inner cutout

                    let num_steps = resolution as usize;

                    let mut outer_points = vec![];
            
                    for idx in 0..=num_steps {
                        let angle = (idx as f64 / num_steps as f64) * 2.0 * PI;
                        outer_points.push(vec2(angle.cos() * outer_radius, angle.sin()*outer_radius));
                    }

                    piece.add_segment(Segment::from_vec(outer_points, allowance.3)); // Bottom seam allowance
                    return piece;
            
                } else if angle >= 1.999 * PI {
                    // Make full circle, with inner cutout

                    let num_steps = resolution as usize;

                    let mut outer_points = vec![];
            
                    for idx in 0..=num_steps {
                        let angle = (idx as f64 / num_steps as f64) * 2.0 * PI;
                        outer_points.push(vec2(angle.cos() * outer_radius, angle.sin()*outer_radius));
                    }

                    piece.add_segment(Segment::from_vec(outer_points, allowance.3)); // Bottom seam allowance
                    // TODO: add inner cutout
                    
                    return piece;

                } else if inner_radius <= allowance.1 {
                    // fraction of circle, no inner cutout
                    // Bottom at x axis
                    let start = -angle/2.0 - PI / 2.0;
                    let stop = start + angle;
                    let diff = stop - start;
                    let num_steps = ((diff / (2.0 * PI) * (resolution as f64)) as usize).max(2);
            
                    let mut outer_points = vec![];
                    for idx in 0..=num_steps {
                        let angle = start + (idx as f64 / num_steps as f64) * diff;
                        let cos_sin = (angle.cos(), angle.sin());
                        outer_points.push(vec2(cos_sin.0 * outer_radius, cos_sin.1 * outer_radius + outer_radius));
                    }

                    // Outer segment
                    piece.add_segment(Segment::from_vec(outer_points, allowance.3));


                    let start_outer = vec2(start.cos() * outer_radius, start.sin() * outer_radius + outer_radius);
                    let end_outer = vec2(stop.cos() * outer_radius, stop.sin() * outer_radius + outer_radius);
                    let inner = vec2(0.0, 0.0 + outer_radius);
                    // Connecting point, right side
                    piece.add_segment(Segment::from_vec(vec![end_outer, inner], allowance.0));

                    // Connecting piece, left side
                    piece.add_segment(Segment::from_vec(vec![inner, start_outer], allowance.2));

                    // todo: flip seam allowance when when segment points "inwards"
                    return piece;
                } else {
                    // Make fraction of a circle, with inner cutout
                    let start = -angle/2.0 - PI / 2.0;
                    let stop = start + angle;
                    let diff = stop - start;
                    let num_steps = ((diff / (2.0 * PI) * (resolution as f64)) as usize).max(2);
            
                    let mut outer_points = vec![];
                    let mut inner_points = vec![];
            
                    for idx in 0..=num_steps {
                        let angle = start + (idx as f64 / num_steps as f64) * diff;
                        let cos_sin = (angle.cos(), angle.sin());
                        outer_points.push(vec2(cos_sin.0 * outer_radius, cos_sin.1 * outer_radius + outer_radius));
                        inner_points.push(vec2(cos_sin.0 * inner_radius, cos_sin.1 * inner_radius + outer_radius))
                    }

                    inner_points.reverse(); // Reverse inner to keep clockwise ordering

                    // Outer segment
                    piece.add_segment(Segment::from_vec(outer_points, allowance.3));


                    let start_outer = vec2(start.cos() * outer_radius, start.sin() * outer_radius + outer_radius);
                    let end_outer = vec2(stop.cos() * outer_radius, stop.sin() * outer_radius + outer_radius);
                    let start_inner = vec2(stop.cos() * inner_radius, stop.sin() * inner_radius + outer_radius);
                    let end_inner = vec2(start.cos() * inner_radius, start.sin() * inner_radius + outer_radius);

                    // Connecting point, right side
                    piece.add_segment(Segment::from_vec(vec![end_outer, start_inner], allowance.0));

                    // Inner curve, top side
                    piece.add_segment(Segment::from_vec(inner_points, allowance.1));

                    // Connecting piece, left side
                    piece.add_segment(Segment::from_vec(vec![end_inner, start_outer], allowance.2));

                    // todo: flip seam allowance when when segment points "inwards"
                    return piece;
                }
            },
            ChuteSectionType::Polygonal(poly) => {
                // To polygonal gore pattern
                let mut piece = PatternPiece::new();
                
                if self.gores < 2 {
                    // Not possible with polygonal shape
                    return piece;
                }

                piece.set_corner_cutout(self.corner_cutout);

                let mut cross_section = self.get_cross_section(resolution, true);

                if cross_section.points.len() < 2 {
                    // Must have at least two points
                    return piece;
                }
                // Duplicate last point
                cross_section.points.push(cross_section.points.last().unwrap().clone());

                let mut right_points = geometry::Points::new();

                // Assume for now that points start from skirt and go towards top.
                let mut y_coord = 0.0;

                let polygon_side_distance = geometry::polygon_center_to_side(self.gores);
                let polygon_side_length = geometry::polygon_edge_len(self.gores);

                for idx in 0..cross_section.points.len()-1 {
                    let this_pt = cross_section.points.get(idx).unwrap();
                    let next_pt = cross_section.points.get(idx+1).unwrap();
                    let diff = (((this_pt.x - next_pt.x) * polygon_side_distance).powi(2) + (this_pt.y - next_pt.y).powi(2)).sqrt();
                    let gore_pt = vec2(polygon_side_length * this_pt.x * 0.5, y_coord);
                    right_points.points.push(gore_pt);
                    y_coord += diff;
                }

                let mut left_points = right_points.mirror_x();
                left_points.points.reverse();
                let top_points = vec![right_points.get_last_point(), left_points.get_first_point()];
                let bottom_points = vec![left_points.get_last_point(), right_points.get_first_point()];


                piece.add_segment(Segment::from_vec(right_points.points, self.seam_allowance.0));
                piece.add_segment(Segment::from_vec(top_points, self.seam_allowance.1));
                piece.add_segment(Segment::from_vec(left_points.points, self.seam_allowance.2));
                piece.add_segment(Segment::from_vec(bottom_points, self.seam_allowance.3));

                return piece;
            }
        }

        PatternPiece::new()
    }

}

impl geometry::ToPoints for ChuteSection {
    fn to_points(&self, resolution: u32) -> geometry::Points {
        self.get_cross_section(resolution, false)
    }
}

// Standard unit combinations for input sliders
#[derive(PartialEq, Clone, Default, Serialize, Deserialize)]
pub enum StandardUnit {
    #[default] UnitLess, // Can be used if other SI units are needed
    MeterFoot,
    MillimeterInch,
    Radian,
    Degree,
}

impl StandardUnit {
    pub fn get_options() -> Vec<Self> {
        vec![
            Self::UnitLess,
            Self::MeterFoot,
            Self::MillimeterInch,
            Self::Radian,
            Self::Degree,
        ]
    }

    pub fn get_general_name(&self) -> String {
        match self {
            Self::UnitLess => "unitless".into(),
            Self::MeterFoot => "m | ft".into(),
            Self::MillimeterInch => "mm | in".into(),
            Self::Radian => "rad".into(),
            Self::Degree => "deg".into(),
        }
    }
}

// Represents a slider that can be used as input
#[derive(PartialEq, Clone, Serialize, Deserialize)]
pub struct InputValue {
    pub id: String, // Unique identifier
    pub description: String, // Short description of what it does
    pub value: f64,
    pub unit: StandardUnit,
    pub range: std::ops::RangeInclusive<f64>, // Range, given in SI base unit
    pub default_value: f64,
}

// These are parameters that are computed using the InputValues. 
// They are evaluated sequentially and may refer to previous computed ParameterValue variables.
#[derive(PartialEq, Clone, Serialize, Deserialize)]
pub struct ParameterValue {
    pub id: String,
    pub expression: String, // Mathematical expression. Value not needed since it's saved in the context
    pub display_unit: StandardUnit, // Only for display purposes. 
}


// Parachute designer interface, implements the relevant UI drawing functions
#[derive(Clone, Serialize, Deserialize)]
pub struct ChuteDesigner {
    name: String,
    gores: u16,
    diameter: f64,

    fabric: FabricSelector,
    instructions: Vec<String>,
    use_global_seam_allowance: bool,
    global_seam_allowance: f64,

    // just for testing
    test_color: [f32; 3],

    input_values: Vec<InputValue>, // Each needs a name, value, range (in m or deg).
    parameter_values: Vec<ParameterValue>, // always in SI units

    chute_sections: Vec<ChuteSection>,

    #[serde(skip)]
    #[serde(default = "ChuteDesigner::default_context")]
    evaluator_context: evalexpr::HashMapContext, // evaluator that handles variables etc. Note: stored value always in SI base unit
}

impl PartialEq for ChuteDesigner {
    fn eq(&self, other: &Self) -> bool {
        if self.name.ne(&other.name) { false }
        else if self.gores.ne(&other.gores) { false }
        else if self.diameter.ne(&other.diameter) { false }
        else if self.fabric.ne(&other.fabric) { false }
        else if self.instructions.ne(&other.instructions) { false }
        else if self.use_global_seam_allowance.ne(&other.use_global_seam_allowance) { false }
        else if self.test_color.ne(&other.test_color) { false }
        else if self.input_values.ne(&other.input_values) { false }
        else if self.parameter_values.ne(&other.parameter_values) { false }
        else { true }
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}


unit! {
    system: uom::si;
    quantity: uom::si::length;

    @unitless: 1.0; "-", "-", "-";
}


impl ChuteDesigner {

    pub fn from_json(s: &str) -> Self {
        serde_json::from_str(s).unwrap_or_else(|_| {
            println!("Error loading file");
            Self::default()
        })
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn options_ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, use_imperial: bool) {

        
        // Number of gores
        /*
        ui::integer_edit_field(ui, &mut self.gores);
        
        ui.checkbox(&mut self.use_global_seam_allowance, "Use global seam allowance");

        ui::dimension_field(ui, &mut self.diameter, use_imperial, 0.0..=10.0);
        ui::number_edit_field(ui, &mut self.diameter);
        */
        ui.add_enabled(self.use_global_seam_allowance, egui::Slider::new::<f64>(&mut self.global_seam_allowance, 0.0..=5.0));
        ui.add_enabled(self.use_global_seam_allowance, egui::Checkbox::new(&mut false, "Cut corners of seam allowance"));

        //egui::widgets::color_picker::color_edit_button_rgb(ui, &mut self.test_color);

        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut self.name)
        });

        for (idx, input_value) in self.input_values.iter_mut().enumerate() {
            
            ui.horizontal(|ui| {
                ui.label(&input_value.id);
                if !input_value.description.is_empty() {
                    ui.button("❓").on_hover_text_at_pointer(&input_value.description);
                }
            });

            ui.horizontal(|ui| {
                ui.label("Value:");
                
                match &input_value.unit {
                    StandardUnit::UnitLess => ui::length_slider(ui, &mut input_value.value, use_imperial, input_value.range.clone(), &unitless, &unitless),
                    StandardUnit::MeterFoot => ui::length_slider(ui, &mut input_value.value, use_imperial, input_value.range.clone(), &length::meter, &length::foot),
                    StandardUnit::MillimeterInch => ui::length_slider(ui, &mut input_value.value, use_imperial, input_value.range.clone(), &length::millimeter, &length::inch),
                    StandardUnit::Degree => ui::length_slider(ui, &mut input_value.value, use_imperial, input_value.range.clone(), &si::angle::degree, &si::angle::degree),
                    StandardUnit::Radian => ui::length_slider(ui, &mut input_value.value, use_imperial, input_value.range.clone(), &si::angle::radian, &si::angle::radian),
                };

                if ui.add_enabled(input_value.value != input_value.default_value, egui::Button::new("↺")).clicked() {
                    input_value.value = input_value.default_value;
                }

            });

            ui.separator();
        }
    }

    pub fn instructions_ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        ui.label(egui::RichText::new("Instructions").font(egui::FontId::proportional(20.0)));
        if ui.button("Add step").clicked() {
            self.instructions.push("Step".to_owned());
        }

        let mut to_delete: Option<usize> = None;

        for (num, step) in self.instructions.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                if ui.button("❌").clicked() {
                    to_delete = Some(num);
                }
                ui.label(format!("{}", num+1));
                ui.separator();

                ui.add(egui::TextEdit::multiline(step).desired_rows(1));
            });
        }

        if let Some(delete_idx) = to_delete {
            self.instructions.remove(delete_idx);
        }
    }

    pub fn draw_cross_section(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, highlighted: Option<u16>) {
        let mut lines = self.get_cross_section();
        lines.append(&mut lines.iter().map(|p| p.mirror_x()).collect());

        self.equal_aspect_plot(ui, frame, &lines, highlighted, "cross_section".into());
    }

    pub fn draw_gores(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, highlighted: Option<u16>) {
        let mut lines = self.get_gores();
        self.equal_aspect_plot(ui, frame, &lines, highlighted, "gore_plot".into());
    }

    pub fn equal_aspect_plot(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, data: &Vec<geometry::Points>, highlighted: Option<u16>, id: String) {
        let mut lines = vec![];

        for (idx,line) in data.iter().enumerate() {
            
            let pts: egui_plot::PlotPoints = line.points.iter().map(|pt| [pt.x, pt.y]).collect();
            let this_line = egui_plot::Line::new(pts).width(2.0).highlight(highlighted == Some(idx as u16));
            
            lines.push(this_line);
        }

        egui_plot::Plot::new(id).height(300.0).data_aspect(1.0).view_aspect(1.5).auto_bounds_x().auto_bounds_y().show(ui, |plot_ui| {
            for line in lines {
                plot_ui.line(line);
            }
        });
    }

    fn has_id_error(id: &String) -> Option<String> {
        if id.contains(char::is_whitespace) {
            return Some("Error: ID cannot contain whitespace characters".into());
        }
        else if id.len() == 0 {
            return Some("Error: ID cannot be empty".into());
        }
        else if !id.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Some("Error: ID must be alphanumeric".into());
        }
        else if !id.chars().next().is_some_and(char::is_alphabetic) {
            return Some("Error: First letter must be alphabetic".into());
        }
        else {
            return None;
        };
    }

    fn default_vars() -> Vec<(String, f64)> {
        vec![
            ("m".into(), 1.0),
            ("mm".into(), 0.001),
            ("yd".into(), 0.9144),
            ("ft".into(), 0.3048),
            ("inch".into(), 0.0254),
            ("rad".into(), 1.0),
            ("pi".into(), PI),
            ("e".into(), core::f64::consts::E),
            ("deg".into(), PI / 180.0)]
    }

    pub fn update_calculations(&mut self) {
        // Call this in order to update the calculation context, only when not rendering geometry_ui

        self.evaluator_context.clear_variables();
        for (name, val) in ChuteDesigner::default_vars() {
            self.evaluator_context.set_value(name, evalexpr::Value::Float(val)).unwrap();
        }

        for (idx, input_value) in self.input_values.iter_mut().enumerate() {
            
            if !evalexpr::Context::get_value(&self.evaluator_context, &input_value.id).is_some()
                && ChuteDesigner::has_id_error(&input_value.id).is_none() {
                self.evaluator_context.set_value(input_value.id.clone(), evalexpr::Value::Float(input_value.value)).unwrap_or_default(); 
            }
        }

        for (idx, parameter) in self.parameter_values.iter_mut().enumerate() {
    
            if !evalexpr::Context::get_value(&self.evaluator_context, &parameter.id).is_some()
                    && ChuteDesigner::has_id_error(&parameter.id).is_none() {

                let computed = evalexpr::eval_number_with_context(&parameter.expression, &self.evaluator_context);
                if computed.is_ok() {
                    let value = computed.unwrap_or_default();

                    self.evaluator_context.set_value(parameter.id.clone(), evalexpr::Value::Float(value));
                }
            }
        }


        for (idx, chute_section) in self.chute_sections.iter_mut().enumerate() {
            chute_section.update_from_context(&self.evaluator_context);
        }

    }

    pub fn geometry_ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, use_imperial: bool) {
        // geometry stuff

        self.evaluator_context.clear_variables();
        for (name, val) in ChuteDesigner::default_vars() {
            self.evaluator_context.set_value(name, evalexpr::Value::Float(val)).unwrap();
        }
        

        ui.columns(2, |columns| {
            let mut ui = &mut columns[0];

            ui.heading("Input variables:");
            ui.separator();
    
            if ui.button("Add input").clicked() {
                self.input_values.push(InputValue { description: "".into(), id: format!("input{}", self.input_values.len()+1), range: 0.0..=10.0, unit: StandardUnit::MeterFoot, value: 1.0, default_value: 1.0})
            }
    
            let mut to_delete: Option<usize> = None;
            let mut to_move: Option<(usize, bool)> = None;
            let num_parameters = self.input_values.len();
    
            for (idx, input_value) in self.input_values.iter_mut().enumerate() {
                
                ui.horizontal(|ui| {
                    if ui.button("❌").clicked() {
                        to_delete = Some(idx);
                    };
                    if ui.add_enabled(idx != 0, egui::Button::new("⬆")).clicked() {
                        to_move = Some((idx, true));
                    }
                    if ui.add_enabled(idx < num_parameters - 1, egui::Button::new("⬇")).clicked() {
                        to_move = Some((idx, false));
                    };
                });
                
                ui.horizontal(|ui| {
                    ui.label("ID:");
                    ui.text_edit_singleline(&mut input_value.id);
                });
                ui.horizontal(|ui| {
                    ui.label("Description");
                    ui.text_edit_singleline(&mut input_value.description);
                });
    
                egui::ComboBox::from_id_source(&input_value.id).width(200.0)
                .selected_text(input_value.unit.get_general_name())
                .show_ui(ui, |ui| {
                    for (_idx, option) in StandardUnit::get_options().iter().enumerate() {
                        ui.selectable_value(&mut input_value.unit, option.clone(), option.get_general_name());
                    }
                });
    
    
                ui.horizontal(|ui| {
                    ui.label("Value:");
                    
                    match &input_value.unit {
                        StandardUnit::UnitLess => ui::length_slider(ui, &mut input_value.value, use_imperial, input_value.range.clone(), &unitless, &unitless),
                        StandardUnit::MeterFoot => ui::length_slider(ui, &mut input_value.value, use_imperial, input_value.range.clone(), &length::meter, &length::foot),
                        StandardUnit::MillimeterInch => ui::length_slider(ui, &mut input_value.value, use_imperial, input_value.range.clone(), &length::millimeter, &length::inch),
                        StandardUnit::Degree => ui::length_slider(ui, &mut input_value.value, use_imperial, input_value.range.clone(), &si::angle::degree, &si::angle::degree),
                        StandardUnit::Radian => ui::length_slider(ui, &mut input_value.value, use_imperial, input_value.range.clone(), &si::angle::radian, &si::angle::radian),
                    }                
                });
    
                // TODO: Add unit conversion selection
                let mut start = *input_value.range.start();
                let mut end = *input_value.range.end();
    
                ui.horizontal(|ui| {
                    ui.label("Range: ");
                    ui::number_edit_field(ui, &mut start);
                    ui.label("to");
                    ui::number_edit_field(ui, &mut end);
                });
                ui.horizontal(|ui| {
                    ui.label("Default value: ");
                    ui::number_edit_field(ui, &mut input_value.default_value);
                });
                input_value.range = start ..= end;
    
                if evalexpr::Context::get_value(&self.evaluator_context, &input_value.id).is_some() {
                    ui.label("Error: Identifier already used");
                }
                else if let Some(msg) = ChuteDesigner::has_id_error(&input_value.id) {
                    ui.label(msg);
                } else {
                    self.evaluator_context.set_value(input_value.id.clone(), evalexpr::Value::Float(input_value.value)).unwrap_or_else(|_| {
                        ui.label("Unable to save value...");
                    })
                }
                ui.separator();
            }
    
            if let Some((idx, direction)) = to_move {
                if idx > 0 && direction {
                    // Swap upwards
                    self.input_values.swap(idx, idx-1);
                } else if idx < (self.input_values.len() - 1) && !direction{
                    self.input_values.swap(idx, idx+1);
                }
            }
    
            
            if let Some(delete_idx) = to_delete {
                self.input_values.remove(delete_idx);
            }



            ui.heading("Computed parameters");
            if ui.button("Add parameter").clicked() {
                self.parameter_values.push(ParameterValue {
                    id: format!("param{}", self.parameter_values.len()+1),
                    expression: "1.0*m+2.0*mm".into(),
                    display_unit: StandardUnit::MeterFoot,
                })
            }
    
            let mut to_delete: Option<usize> = None;
            let mut to_move: Option<(usize, bool)> = None; // Option containing index and true if moving up and false if down
    
            ui.push_id("paramtable", |ui| {
                egui_extras::TableBuilder::new(ui)
                .striped(true)
                .column(egui_extras::Column::auto())
                .column(egui_extras::Column::auto().at_least(60.0).resizable(true))
                .column(egui_extras::Column::auto().at_least(120.0).resizable(true))
                .column(egui_extras::Column::remainder())
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.label(egui::RichText::new("Edit").strong());
                    });
                    header.col(|ui| {
                        ui.label(egui::RichText::new("ID").strong());
                    });
                    header.col(|ui| {
                        ui.label(egui::RichText::new("Expression").strong());
                    });
                    header.col(|ui| {
                        ui.label(egui::RichText::new("Result [si unit]").strong());
                    });
                })
                .body(|mut body| {
                    let num_parameters = self.parameter_values.len();
                    for (idx, parameter) in self.parameter_values.iter_mut().enumerate() {
    
    
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.horizontal(|ui| {
                                    if ui.button("❌").clicked() {
                                        to_delete = Some(idx);
                                    };
                                    if ui.add_enabled(idx != 0, egui::Button::new("⬆")).clicked() {
                                        to_move = Some((idx, true));
                                    }
                                    if ui.add_enabled(idx < num_parameters - 1, egui::Button::new("⬇")).clicked() {
                                        to_move = Some((idx, false));
                                    };
                                });
                            });
                            row.col(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut parameter.id).clip_text(false));
                            });
                            row.col(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut parameter.expression).clip_text(false));
                            });
                            row.col(|ui| {
                                
                                if evalexpr::Context::get_value(&self.evaluator_context, &parameter.id).is_some() {
                                    ui.label("Error: Identifier already used");
                                }
                                else if let Some(msg) = ChuteDesigner::has_id_error(&parameter.id) {
                                    ui.label(msg);
                                }
                                else {
                                    let computed = evalexpr::eval_number_with_context(&parameter.expression, &self.evaluator_context);
                                    if computed.is_ok() {
                                        let value = computed.unwrap_or_default();
                                        ui.label(format!("{:}", (value * 100_000_000.0).round() / 100_000_000.0));
                    
                                        self.evaluator_context.set_value(parameter.id.clone(), evalexpr::Value::Float(value));
                                    } else {
                                        ui.label(format!("Error: {:?}", computed));
                                    }
                                }
                            });
                            
                        });
                    }
                });    
            });
    
    
            if let Some((idx, direction)) = to_move {
                if idx > 0 && direction {
                    // Swap upwards
                    self.parameter_values.swap(idx, idx-1);
                } else if idx < (self.parameter_values.len() - 1) && !direction{
                    self.parameter_values.swap(idx, idx+1);
                }
            }
    
            if let Some(delete_idx) = to_delete {
                self.parameter_values.remove(delete_idx);
            }
    
    
            ui.add(egui::Hyperlink::from_label_and_url("Info about builtin functions", "https://docs.rs/evalexpr/latest/evalexpr/#builtin-functions"));
            
            ui.separator();
    
            ui = &mut columns[1];
            


            ui.heading("Cross-section geometry");

            ui.horizontal(|ui| {
                if ui.button("Add polygonal geometry").clicked() {
                    self.chute_sections.push(ChuteSection::new_polygonal());
                }
                
                if ui.button("Add circular band").clicked() {
                    self.chute_sections.push(ChuteSection::new_circular());
                }
            });
    
    
            // Parachute section
    
            let mut to_delete: Option<usize> = None;
            let mut to_move: Option<(usize, bool)> = None; // Option containing index and true if moving up and false if down
            let num_parameters = self.chute_sections.len();
            let mut focus_idx: Option<u16> = None;

            for (idx, chute_section) in self.chute_sections.iter_mut().enumerate() {
                ui.separator();
                let res = egui::Frame::none().stroke(egui::Stroke::new(1.0, egui::Color32::GRAY)).inner_margin(10.0).outer_margin(5.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            if ui.button("❌").clicked() {
                                to_delete = Some(idx);
                            };
                            if ui.add_enabled(idx != 0, egui::Button::new("⬆")).clicked() {
                                to_move = Some((idx, true));
                            }
                            if ui.add_enabled(idx < num_parameters - 1, egui::Button::new("⬇")).clicked() {
                                to_move = Some((idx, false));
                            };
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            chute_section.ui(ui, frame, use_imperial, &self.evaluator_context, idx as u16);
                        });
                    });
                }).response;

                if res.hovered() {
                    focus_idx = Some(idx as u16);
                }
            }
    
            if let Some((idx, direction)) = to_move {
                if idx > 0 && direction {
                    // Swap upwards
                    self.chute_sections.swap(idx, idx-1);
                } else if idx < (self.chute_sections.len() - 1) && !direction{
                    self.chute_sections.swap(idx, idx+1);
                }
            }
    
            if let Some(delete_idx) = to_delete {
                self.chute_sections.remove(delete_idx);
            }
    
            ui.separator();
    
            self.draw_cross_section(ui, frame, focus_idx);

            self.draw_gores(ui, frame, focus_idx);

        });

    }

    pub fn experiment_ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, use_imperial: bool) {
        // For measurement data
        // E.g. input velocity/density and get CD
        ui.label("Descent rate");
        ui::length_slider(ui, &mut self.diameter, use_imperial, 0.0..=10.0, &si::velocity::meter_per_second, &si::velocity::foot_per_second);
    }

    pub fn get_cross_section(&self) -> Vec<geometry::Points> {
        // get 2D cross section for display purposes
        let mut result = vec![];

        for chute_section in &self.chute_sections {
            // use relatively low resolution at 30 points
            result.push(chute_section.get_cross_section(30, false));
        }

        result
    }

    pub fn get_gores(&self) -> Vec<geometry::Points> {
        let mut result = vec![];

        for chute_section in &self.chute_sections {
            let mut piece = chute_section.to_pattern_piece(80);
            piece.compute();
            result.push( geometry::Points::from_vec(piece.computed_points));
        }

        result
    }

    pub fn get_3d_data(&self) -> Vec<three_d::CpuMesh> {
        // Go through and generate the correct colors and mesh for 3D rendering...
        let mut result = vec![];

        // Note: This generates the mesh every single time, without checking if it's identical to the last time.

        // Save everything in one mesh
        let mut triangle_indices: Vec<u32> = vec![];
        let mut new_colors: Vec<three_d::Srgba> = vec![];
        let mut chute_coords: Vec<three_d::Vector3<f32>> = vec![];

        let mut idx_offset: u32 = 0;

        for section in &self.chute_sections {
            //let mut chute_cross = vec![];

            let chute_cross = section.get_cross_section(60, true);

            if chute_cross.points.len() < 2 {
                continue;
            }

            let mut chute_cross: Vec<three_d::Vector2<f32>> = chute_cross.points.iter().map(|pt| three_d::vec2(pt.x as f32, pt.y as f32)).collect();

            let num_gores = section.gores as usize;

            if num_gores == 0 {
                // If zero
                continue;
            }


            for idx in 0..num_gores {
                let z_angle = idx as f32 / (num_gores as f32) * core::f32::consts::PI * 2.0;
                let cos_sin = (z_angle.cos(), z_angle.sin());
                // Append twice to allow per gore coloring
                chute_coords.append(&mut chute_cross.iter().map(|&pt| three_d::vec3(pt.x * cos_sin.0, pt.y, pt.x * cos_sin.1) ).collect::<Vec<_>>());
                chute_coords.append(&mut chute_cross.iter().map(|&pt| three_d::vec3(pt.x * cos_sin.0, pt.y, pt.x * cos_sin.1) ).collect::<Vec<_>>());
            }


            let num_points_per_gore = chute_cross.len();

            // Generate triangles. 
            for gore_idx in 0..num_gores {
                // Each gore
                for point_idx in 0..(num_points_per_gore-1) {
                    // Each point on the left of that gore

                    // Get four points forming a square
                    let pt_left0 = ((gore_idx*2+1) * num_points_per_gore + point_idx) as u32;
                    let pt_left1 = pt_left0 + 1;
                    let pt_right0 = ((((gore_idx*2+1) + 1) % (num_gores * 2)) * num_points_per_gore + point_idx) as u32;
                    let pt_right1 = pt_right0 + 1;
                    
                    // Two triangles forming a square
                    // Order counterclockwise
                    triangle_indices.append(&mut vec![pt_left0+idx_offset, pt_right0+idx_offset, pt_left1+idx_offset,
                                                     pt_right0+idx_offset, pt_right1+idx_offset, pt_left1+idx_offset]);
                }            
            }


            let color_map: Vec<three_d::Srgba> = if section.colors.is_empty() {
                vec![three_d::Srgba::new(255, 79, 0, 255)]
            } else {
                section.colors.iter().map(|rgb| ui::rgb_to_srgba(rgb)).collect()
            };

            let num_colors = color_map.len();

            for j in 0..num_points_per_gore {
                new_colors.push(color_map.get(0).unwrap().clone());
            }
    
            for gore_num in 0..(num_gores-1) {
                for j in 0..num_points_per_gore {
                    new_colors.push(color_map.get((gore_num + 1) % num_colors).unwrap().clone());
                    new_colors.push(color_map.get((gore_num + 1) % num_colors).unwrap().clone());
                }
            }

            for j in 0..num_points_per_gore {
                new_colors.push(color_map.get(0).unwrap().clone());
            }
            
            idx_offset = chute_coords.len() as u32;
    
            //for idx in 0..cpu_mesh.vertex_count() {
            //    new_colors.push(Srgba::new((((idx % (5 * num_points_per_gore))/num_points_per_gore * 255) % 256) as u8 , 0 as u8, 0 as u8, 255))
            //}

        }

        if chute_coords.is_empty() {

            // If no geometry present, add a triangle
            chute_coords.push(three_d::vec3(0.0, 0.0, 0.0));
            chute_coords.push(three_d::vec3(0.0, 0.0, 0.0));
            chute_coords.push(three_d::vec3(0.0, 0.0, 0.0));

            new_colors.push(three_d::Srgba::new(0, 0, 0, 0));
            new_colors.push(three_d::Srgba::new(0, 0, 0, 0));
            new_colors.push(three_d::Srgba::new(0, 0, 0, 0));

            triangle_indices.append(&mut vec![0, 1, 2]);
        }

        let mut cpu_mesh = three_d::CpuMesh {
            positions: three_d::Positions::F32(chute_coords),
            indices: three_d::Indices::U32(triangle_indices),
            ..Default::default()
        };

        cpu_mesh.colors = Some(new_colors);
    
        cpu_mesh.compute_normals();
        
        result.push(cpu_mesh);

        result
    }

    pub fn export_dxf(&mut self) {

        // Create a new DXF drawing
        let mut drawing = Drawing::new();

        let mut x_offset = 0.0;

        let add_text = true;

        for (idx,chute_section) in self.chute_sections.iter().enumerate() {
            let mut piece = chute_section.to_pattern_piece(360); // High resolution for export
            
            // Create a polyline entity for the triangle
            let mut polyline = Polyline::default();
            polyline.set_is_closed(true); // Closed polyline for a triangle
            
            piece.compute();

            println!("Total area: {} including seams, {} not including seams (m2)", piece.get_area(true) * chute_section.gores as f64, piece.get_area(false) * chute_section.gores as f64);
            
            let gore_points = geometry::Points::from_vec(piece.computed_points);
            let (min, max) = gore_points.bounds();

            let width = max.x - min.x;

            for point in &gore_points.points {
                let vertex = Vertex::new(dxf::Point::new(point.x + x_offset - min.x, point.y - min.y, 0.0));
                polyline.add_vertex(&mut drawing, vertex);
            }

            let mut label = Text::default();
            label.value = format!("#{}(x{})", idx + 1, chute_section.gores);
            label.horizontal_text_justification = dxf::enums::HorizontalTextJustification::Left;
            label.location = dxf::Point::new(x_offset, -0.2, 0.0);
            //label.second_alignment_point = dxf::Point::new(x_offset + width, -0.2, 0.0);
            label.text_height = 0.04;

            x_offset += width + 0.1; // 10 cm between
            
            // Add the polyline to the drawing
            drawing.add_entity(Entity::new(EntityType::Polyline(polyline)));

            drawing.add_entity(Entity::new(EntityType::Text(label)));
        }

        drawing.header.default_drawing_units = dxf::enums::Units::Meters;

        // Save the drawing to a DXF file
        drawing.save_file("gores.dxf").unwrap();

        println!("Gores saved to gores.dxf");
        // TODO: location select


    }

    pub fn default_context() -> evalexpr::HashMapContext {
        evalexpr::context_map! {
            "m" => 1.0,
            "mm" => 0.001,
            "yd" => 0.9144,
            "ft" => 0.3048,
            "in" => 0.0254,
            "rad" => 1.0,
            "pi" => PI,
            "e" => core::f64::consts::E,
            "deg" => PI / 180.0,
            "ln" => Function::new(|arg| Ok(evalexpr::Value::Float(arg.as_float()?.ln())))
        }.unwrap()
    }
}


impl Default for ChuteDesigner {
    fn default() -> Self {
        let context = Self::default_context();

        // TODO: make math functions work without prefix

        let input1 = InputValue {description: "".into(), id: "input1".into(), range: 0.0..=10.0, unit: StandardUnit::MeterFoot, value: 0.0, default_value: 0.0};
        let input2 = InputValue {description: "".into(), id: "input2".into(), range: 0.0..=10.0, unit: StandardUnit::MeterFoot, value: 0.0, default_value: 0.0};

        let input3 = InputValue {description: "Parachute Diameter".into(), id: "diameter".into(), range: 0.0..=10.0, unit: StandardUnit::MeterFoot, value: 1.0, default_value: 1.0};
        let input4 = InputValue {description: "height / diameter of parachute".into(), id: "height_ratio".into(), range: 0.0..=1.0, unit: StandardUnit::UnitLess, value: 0.7, default_value: 0.7};
        let input5 = InputValue {description: "vent_diameter / diameter of parachute".into(), id: "vent_ratio".into(), range: 0.0..=1.0, unit: StandardUnit::UnitLess, value: 0.2, default_value: 0.2};


        let param1 = ParameterValue {display_unit: StandardUnit::MeterFoot, id: "param1".into(), expression: "input1*2".into()};
        let param2 = ParameterValue {display_unit: StandardUnit::MeterFoot, id: "param2".into(), expression: "input2*2".into()};
        let param3 = ParameterValue {display_unit: StandardUnit::MeterFoot, id: "param3".into(), expression: "param1+param2".into()};

        let mut section1 = ChuteSection::new_circular();
        match &mut section1.section_type {
            ChuteSectionType::Circular(sec) => {
                sec.expressions[0] = "vent_ratio*diameter".into();
                sec.expressions[1] = "height_ratio*diameter".into();
                sec.expressions[2] = "diameter".into();
                sec.expressions[3] = "0".into();
            },
            _ => {}
        }

        Self { 
            name: "Untitled Parachute".to_owned(),
            gores: 8,
            diameter: 1.0,
            instructions: vec!["Cut out fabric".to_owned()],
            fabric: FabricSelector::new(),
            use_global_seam_allowance: true,
            global_seam_allowance: 0.01,
            test_color: [0.0, 0.0, 0.0],
            input_values: vec![input1, input2, input3, input4, input5],
            parameter_values: vec![param1, param2, param3],
            evaluator_context: context,
            chute_sections: vec![section1],
        }
    }
}


fn example_write_dxf() -> Result<(), Box<dyn std::error::Error>> {
    // Define the vertices of the triangle
    let vertices = [(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)];

    // Create a new DXF drawing
    let mut drawing = Drawing::new();

    // Create a polyline entity for the triangle
    let mut polyline = Polyline::default();
    polyline.set_is_closed(true); // Closed polyline for a triangle

    // Add vertices to the polyline
    for &(x, y) in &vertices {
        let vertex = Vertex::new(dxf::Point::new(x, y, 0.0));
        polyline.add_vertex(&mut drawing, vertex);
    }

    // Add the polyline to the drawing
    drawing.add_entity(Entity::new(EntityType::Polyline(polyline)));

    // Save the drawing to a DXF file
    drawing.save_file("triangle.dxf")?;

    println!("Triangle saved to triangle.dxf");

    Ok(())
}


// An array of 2D points representing a line in a pattern piece. Seam allowance is constant throughout a segment
// All dimensions given in mm
#[derive(Clone, Debug)]
struct Segment {
    points: Vec<na::Vector2<f64>>,
    seam_allowance: f64,
}

impl Segment {
    fn new() -> Self {
        Self { points: vec![], seam_allowance: 0.0 }
    }

    fn from_vec(points: Vec<na::Vector2<f64>>, seam_allowance: f64) -> Self {
        Self { points: points, seam_allowance: seam_allowance }
    }

    fn new_with_allowance(seam_allowance: f64) -> Self {
        Self { points: vec![], seam_allowance: seam_allowance }
    }

    fn set_seam_allowance(&mut self, seam_allowance: f64) {
        self.seam_allowance = seam_allowance;
    }

    fn add_point(&mut self, point: na::Vector2<f64>) {
        self.points.push(point);
    }

    fn add_point_xy(&mut self, x: f64, y: f64) {
        self.points.push(na::Vector2::new(x, y));
    }

    fn mirror_x(&self) -> Self {
        // returns a copy mirrored around the X axis
        let mut new = self.clone();
        for point in new.points.iter_mut() {
            point.x = -point.x;
        }
        new
    }

    fn reverse(&self) -> Self {
        let mut new = self.clone();
        new.points.reverse();
        new
    }

    fn scale(&mut self, scale_x: f64, scale_y: f64) {
        // Scale around origin
        for point in self.points.iter_mut() {
            point.x = point.x * scale_x;
            point.y = point.y * scale_y;
        }
    }

    fn get_point(&self, idx: usize) -> na::Vector2<f64> {
        self.points[idx].clone()
    }

    fn get_first_point(&self) -> na::Vector2<f64> {
        self.points.first().unwrap().clone()
    }

    fn get_last_point(&self) -> na::Vector2<f64> {
        self.points.last().unwrap().clone()
    }
}

pub struct PatternPiece {
    segments: Vec<Segment>,
    points: Vec<na::Vector2<f64>>, // Points not including seams
    computed_points: Vec<na::Vector2<f64>>, // Final points including seams
    fabric_area: f64, // Area in mm2, includes seam allowances
    chute_area: f64, // Area in mm2, not including seam allowances
    count: u16, // Number of duplicate pattern pieces
    corner_cutout: bool,
    name: String,
}

// Generic arbitrary 2D pattern piece
// Segments are used to allow for different seam allowances
// All the segments are connected together
// Segments NEED to be defined in counterclockwise direction
// Points connecting pieces can be duplicated, but not necessary
// Edge joining two segments is given seam allowance of previous segment
impl PatternPiece {
    fn new() -> PatternPiece {
        Self { segments: vec![], points: vec![], computed_points: vec![], fabric_area: 0.0, chute_area: 0.0, name: "pattern".into(), count: 1, corner_cutout: true}
    }

    fn add_segment(&mut self, seg: Segment) {
        self.segments.push(seg);
    }

    fn set_corner_cutout(&mut self, cutout: bool) {
        self.corner_cutout = cutout;
    }

    fn compute(&mut self) {
        // Compute seam allowances etc
        self.computed_points = vec![];
        self.points = vec![];


        // only process non_empty segments
        let mut segments = vec![];

        for seg in &self.segments {
            if !seg.points.is_empty() {
                segments.push(seg.clone());
            }
        }

        if segments.is_empty() {
            return;
        }

        for idx in 1..=segments.len() {
            // Delete last point in segment until it's unique
            
            let idx_this = idx % segments.len();
            let idx_next = (idx + 1) % segments.len();

            let next_start = segments[idx_next].points.first().unwrap_or(&vec2(f64::NAN, f64::NAN)).clone();

            while segments[idx_this].points.last().is_some_and(|pt| (pt - next_start).norm_squared() < 1e-6) {
                segments[idx_this].points.pop();
            }
        }

        // Remove empty segments again
        segments.retain(|seg| !seg.points.is_empty());

        let mut allowance_prev = segments.last().unwrap().seam_allowance;
        let mut allowance_next = segments.first().unwrap().seam_allowance;

        let mut prev_pt = segments.last().unwrap().points.last().unwrap().clone(); // final point

        for (seg_idx, seg) in segments.iter().enumerate() {
            self.points.append(&mut seg.points.clone()); // Base shape without seam allowance
            let seg_idx_next = (seg_idx + 1) % segments.len();

            for idx in 0..seg.points.len() {
                // Corner is first point in segment
                let is_corner = idx == 0;
                
                if is_corner && seg_idx != 0 {
                    allowance_prev = allowance_next;
                    allowance_next = seg.seam_allowance;
                }

                let p0 = prev_pt;
                let p1 = seg.get_point(idx);
                let p2 = if idx < (seg.points.len() - 1) { seg.get_point(idx + 1) } else { segments.get(seg_idx_next).unwrap().get_first_point() };
                prev_pt = p1;
                //println!("points: {:?}, {:?}, {:?}", p0, p1, p2);

                // Based on https://stackoverflow.com/questions/3749678/expand-fill-of-convex-polygon
                let v01 = p1 - p0;
                let v12 = p2 - p1;

                let v01_norm = v01.norm();
                let v12_norm = v12.norm();

                // Rotate clockwise and normalise
                let n01 = vec2(v01.y, -v01.x) / v01_norm;
                let n12 = vec2(v12.y, -v12.x) / v12_norm;

                let d = v12.dot(&n01);

                if v01_norm < 1e-6 && v12_norm < 1e-6 {
                    // All points are at the same position
                    self.computed_points.push(p0);
                }
                else if d.abs() < 1e-6 || v01_norm < 1e-6 || v12_norm < 1e-6 {
                    // Line is parallel, just offset with n01 with average offset

                    let used_norm = if v01_norm < 1e-6 { n12 } else { n01 };

                    let p0_offset_prev = p0 + used_norm * allowance_prev;
                    let p0_offset_next = p0 + used_norm * allowance_next;

                    if self.corner_cutout {
                        // Add both points
                        self.computed_points.push(p0_offset_prev);
                        self.computed_points.push(p0_offset_next);
                    } else {
                        // Average
                        self.computed_points.push((p0_offset_prev + p0_offset_next) * 0.5);
                    }
                } else {
                    // Add to original points to define line
                    let p0_offset_prev = p0 + n01 * allowance_prev;
                    let p1_offset_prev = p1 + n01 * allowance_prev;
                    let p1_offset_next = p1 + n12 * allowance_next;
                    let p2_offset_next = p2 + n12 * allowance_next;

                    // Find intersection
                    if self.corner_cutout && (is_corner) {
                        // Make corner cutout
                        self.computed_points.push(p1_offset_prev);
                        self.computed_points.push(p1);
                        self.computed_points.push(p1_offset_next);
                    } else {
                        let a1 = p1_offset_prev.x - p0_offset_prev.x;
                        let b1 = p1_offset_next.x - p2_offset_next.x;
                        let c1 = p1_offset_next.x - p0_offset_prev.x;
    
                        let a2 = p1_offset_prev.y - p0_offset_prev.y;
                        let b2 = p1_offset_next.y - p2_offset_next.y;
                        let c2 = p1_offset_next.y - p0_offset_prev.y;

                        let t = (b1*c2 - b2*c1) / (a2*b1 - a1*b2);

                        let new_pt = vec2(p0_offset_prev.x + t * (p1_offset_prev.x - p0_offset_prev.x), p0_offset_prev.y + t * (p1_offset_prev.y - p0_offset_prev.y));
                        self.computed_points.push(new_pt);           
                    }
                }
                allowance_prev = allowance_next;

            }   
        }

        if let Some(first_pt) = self.computed_points.first() {
            self.computed_points.push(first_pt.clone()); // wrap around
        }
    }

    fn save_dxf(&mut self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.compute();

        // Create a new DXF drawing
        let mut drawing = Drawing::new();

        // Create a polyline entity for the triangle
        let mut polyline = Polyline::default();
        polyline.set_is_closed(true); // Closed polyline for a triangle

        // Add vertices to the polyline
        for point in self.computed_points.iter() {
            let vertex = Vertex::new(dxf::Point::new(point.x, point.y, 0.0));
            polyline.add_vertex(&mut drawing, vertex);
        }

        // Add the polyline to the drawing
        drawing.add_entity(Entity::new(EntityType::Polyline(polyline)));

        // Save the drawing to a DXF file
        drawing.save_file(filename)?;

        println!("Triangle saved to {}", filename);

        Ok(())

    }

    fn get_area(&self, including_seams: bool) -> f64 {
        let mut points = if including_seams { self.computed_points.clone() } else { self.points.clone() };
        points.push(points.first().unwrap().clone()); // Wrap around

        let mut sum = 0.0;
        // https://en.wikipedia.org/wiki/Shoelace_formula
        for point_pair in points.windows(2) {
            //println!("{:?}", point_pair);
            sum += point_pair[0].x * point_pair[1].y - point_pair[0].y * point_pair[1].x;
        }
        sum/2.0
    }
}

struct PatternPieceCollection {
    pieces: Vec<PatternPiece>, // Pattern piece and number of each
}

impl PatternPieceCollection {
    fn new() -> Self {
        Self { pieces: vec![] }
    }

    fn get_area(&self, including_seams: bool) -> f64 {
        let mut total_area = 0.0;

        for piece in self.pieces.iter() {
            total_area += piece.get_area(including_seams) * (piece.count as f64);
        }

        total_area
    }
}

// Standard gore geometry with four sides and straight top/bottom section
// If coords_left is None, the coords are mirrored on the left side
// Note: Coordinates go from bottom to top for BOTH left and right sections
struct Gore {
    coords_right: Segment,
    coords_left: Segment,
    seam_allowances: (f64, f64, f64, f64), // Seams. Right, top, left, bottom
}

impl Gore {
    fn new(coords_right: Segment, coords_left: Segment, seam_allowances: (f64, f64, f64, f64)) -> Self {
        // Coords are defined from bottom to top
        Self { coords_right: coords_right, coords_left: coords_left, seam_allowances: seam_allowances}
    }

    fn new_symmetric(coords_right: Segment, seam_allowances: (f64, f64, f64, f64)) -> Self {
        Gore::new(coords_right.clone(), coords_right.clone().mirror_x(), seam_allowances)
    }

    fn get_pattern_piece(&self, corner_cutout: bool) -> PatternPiece {
        let mut piece = PatternPiece::new();
        // Add right segment, excluding top point

        let mut segment_right = self.coords_right.clone();
        segment_right.set_seam_allowance(self.seam_allowances.0);

        let mut segment_left = self.coords_left.reverse();
        segment_left.set_seam_allowance(self.seam_allowances.2);
        
        let mut segment_top = Segment::new_with_allowance(self.seam_allowances.1);
        segment_top.add_point(segment_right.get_last_point());
        segment_top.add_point(segment_left.get_first_point());

        let mut segment_bottom = Segment::new_with_allowance(self.seam_allowances.3);
        segment_bottom.add_point(segment_left.get_last_point());
        segment_bottom.add_point(segment_right.get_first_point());
        
        piece.add_segment(segment_right);
        piece.add_segment(segment_top);
        piece.add_segment(segment_left);
        piece.add_segment(segment_bottom);

        piece
    }
}


// Trait for different parachute types
trait Parachute {
    fn get_pieces(&self) -> PatternPieceCollection;
    fn get_fabric_area(&self) -> f64;
    fn get_parachute_area(&self) -> f64;
    fn get_projected_area(&self) -> f64;
    fn get_cd(&self) -> f64; // Drag coefficient, based on parachute_area
    fn get_line_length(&self) -> f64;
}

struct ParachuteProject {
    chute: Box<dyn Parachute>
}


// Suspension line stuff
struct SuspensionLine {
    rating_newtons: f64,
    linear_density_g_m: f64,
    name: String,
}

#[derive(Default, Clone, PartialEq, Serialize, Deserialize)]
struct Fabric {
    area_density_gsm: f64,
    name: String
}

impl Fabric {
    fn new(gsm: f64, name: &str) -> Self {
        Self { area_density_gsm: gsm, name: name.to_owned() }
    }

    fn get_name_weight(&self, imperial: bool) -> String {
        if imperial {
            format!("{} ({:.1} oz)", self.name, self.area_density_gsm / 33.906)
        } else {
            format!("{} ({:.0} gsm)", self.name, self.area_density_gsm)
        }
    }

}

#[derive(PartialEq, Clone, Serialize, Deserialize)]
struct FabricSelector {
    modified: bool,
    selected_fabric: Fabric,
    fabric_options: Vec<Fabric>,
}

impl FabricSelector {
    fn new() -> Self {

        let mut options = vec![];

        options.push(Fabric::new(38.0, "Ripstop nylon"));
        options.push(Fabric::new(48.0, "Ripstop nylon"));
        options.push(Fabric::new(67.0, "Ripstop nylon"));

        let default_fabric = Fabric::new(38.0, "Ripstop nylon");

        Self { modified: false, fabric_options: options, selected_fabric: default_fabric }
    }


    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, use_imperial: bool, id: u16) {
        egui::ComboBox::from_id_source(id)
            .width(200.0)
            .selected_text(self.selected_fabric.get_name_weight(use_imperial))
            .show_ui(ui, |ui| {
                for (_idx, option) in self.fabric_options.iter().enumerate() {
                    ui.selectable_value(&mut self.selected_fabric, option.clone(), option.get_name_weight(use_imperial));
                }
            }
        );
    }
}

impl Default for FabricSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::chute::parachute::{Segment, PatternPiece, ChuteSection};
    extern crate nalgebra as na;

    #[test]
    fn test_save_dxf() {
        let mut seg = Segment::new();
        seg.add_point_xy(0.0, 0.0);
        seg.add_point_xy(1.0, 0.0);
        seg.add_point_xy(0.5, 2.0);

        let mut pat = PatternPiece::new();
        pat.add_segment(seg);
        pat.compute();
        pat.save_dxf("tall_triangle.dxf").unwrap();
    }

    #[test]
    fn test_mirror_scale() {
        let mut seg = Segment::new();
        seg.add_point_xy(0.0, 0.0);
        seg.add_point_xy(1.0, 2.0);

        println!("{:?}", seg);
        seg.mirror_x();
        assert_eq!(seg.points[1], na::Vector2::new(-1.0, 2.0));
        println!("{:?}", seg);
        seg.scale(2.0, 3.0);
        assert_eq!(seg.points[1], na::Vector2::new(-2.0, 6.0));
    }

    #[test]
    fn test_area() {
        let mut seg = Segment::new();
        // Funky triangle, should give area of 2
        seg.add_point_xy(0.0, 0.0);
        seg.add_point_xy(1.0, 0.0);
        seg.add_point_xy(-0.5, 4.0);

        let mut pat = PatternPiece::new();
        pat.add_segment(seg);
        pat.compute();

        assert_eq!(pat.get_area(false), 1.0 * 4.0 * 0.5)
    }

    #[test]
    fn test_seam_allowance() {
        let section = ChuteSection::new_circular();
        let mut pat = section.to_pattern_piece(20);

        pat.corner_cutout = false;
        pat.compute();

        println!("{:?}", pat.computed_points);

    }
}