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

use std::f64::consts::PI;

use uom::si::{self, length};

// Represents a band. Can contain multiple geometries representing the cross section, but is symmetrical and has a fixed number of gores
pub struct ChuteBand {
    
}

// Standard unit combinations for input sliders
#[derive(PartialEq, Clone)]
pub enum StandardUnit {
    UnitLess, // Can be used if other SI units are needed
    MeterFoot,
    MillimeterInch,
    Radian,
    Degree,
}

// Represents a slider that can be used as input
#[derive(PartialEq, Clone)]
pub struct InputValue {
    pub id: String, // Unique identifier
    pub description: String, // Short description of what it does
    pub value: f64,
    pub unit: StandardUnit,
    pub range: std::ops::RangeInclusive<f64>, // Range, given in SI base unit
}

// These are parameters that are computed using the InputValues. 
// They are evaluated sequentially and may refer to previous computed ParameterValue variables.
#[derive(PartialEq, Clone)]
pub struct ParameterValue {
    pub id: String,
    pub expression: String, // Mathematical expression. Value not needed since it's saved in the context
    pub display_unit: StandardUnit, // Only for display purposes. 
}

// TODO: Add warning when id used multiple times
// TODO: Add way to move parameters up and down


// Parachute designer interface, implements the relevant UI drawing functions
#[derive(Clone)]
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
    parametric_expressions: Vec<ParameterValue>,

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
        else if self.parametric_expressions.ne(&other.parametric_expressions) { false }
        else { true }
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}


unit! {
    system: uom::si;
    quantity: uom::si::length;

    @sheppey: 1408.176; "sheppey", "sheppey", "sheppeys";
}


impl ChuteDesigner {
    pub fn options_ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, use_imperial: bool) {
        
        // Number of gores
        ui::integer_edit_field(ui, &mut self.gores);

        ui::length_slider(ui, &mut self.diameter, use_imperial, 0.0..=6.0, &length::meter, &length::microinch);
        ui::length_slider(ui, &mut self.diameter, use_imperial, 0.0..=6.0, &length::meter, &length::inch);
        ui::length_slider(ui, &mut self.diameter, use_imperial, 0.0..=10.0, &length::millimeter, &sheppey);


        ui.checkbox(&mut self.use_global_seam_allowance, "Use global seam allowance");

        ui.add_enabled(self.use_global_seam_allowance, egui::Slider::new::<f64>(&mut self.global_seam_allowance, 0.0..=5.0));
        ui.add_enabled(self.use_global_seam_allowance, egui::Checkbox::new(&mut false, "Cut corners of seam allowance"));

        egui::widgets::color_picker::color_edit_button_rgb(ui, &mut self.test_color);


        self.fabric.ui(ui, frame, use_imperial);

        self.draw_cross_section(ui, frame);
    }

    pub fn instructions_ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        ui.label(egui::RichText::new("Instructions").font(egui::FontId::proportional(20.0)));
        if ui.button("Add step").clicked() {
            self.instructions.push("Step".to_owned());
        }

        let mut to_delete: Option<usize> = None;

        for (num, step) in self.instructions.iter().enumerate() {
            if ui.selectable_label(false, format!("{}: {}", num + 1, step)).clicked() {
                to_delete = Some(num);
            }
        }

        if let Some(delete_idx) = to_delete {
            self.instructions.remove(delete_idx);
        }
    }

    pub fn draw_cross_section(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        // Generate points
        let cross = geometry::Points::from_vec((0..80).map(|ang| {
            na::Vector2::new((ang as f64 * PI/180.0).cos() * self.diameter * 0.5,(ang as f64 * PI/180.0).sin() * 0.7 * self.diameter * 0.5)
        }).collect());

        let cross1 = geometry::Points::from_vec((0..80).map(|ang| {
            na::Vector2::new(-(ang as f64 * PI/180.0).cos() * self.diameter * 0.5,(ang as f64 * PI/180.0).sin() * 0.7 * self.diameter * 0.5)
        }).collect());

        let lines = vec![cross, cross1];

        self.equal_aspect_plot(ui, frame, &lines, Some(1));
    }

    pub fn equal_aspect_plot(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, data: &Vec<geometry::Points>, highlighted: Option<u16>) {
        let mut lines = vec![];

        for (idx,line) in data.iter().enumerate() {
            
            let pts: egui_plot::PlotPoints = line.points.iter().map(|pt| [pt.x, pt.y]).collect();
            let this_line = egui_plot::Line::new(pts).width(2.0).highlight(highlighted == Some(idx as u16));
            
            lines.push(this_line);
        }

        egui_plot::Plot::new("cross_section").height(300.0).data_aspect(1.0).view_aspect(1.5).auto_bounds_x().auto_bounds_y().show(ui, |plot_ui| {
            for line in lines {
                plot_ui.line(line);
            }
        });
    }

    fn validate_identifier(id: &String, ui: &mut egui::Ui) -> bool {
        if id.contains(char::is_whitespace) {
            ui.label("Error: ID cannot contain whitespace characters");
        }
        else if id.len() == 0 {
            ui.label("Error: ID cannot be empty");
        }
        else if !id.chars().all(char::is_alphanumeric) {
            ui.label("Error: ID must be alphanumeric");
        }
        else if !id.chars().next().is_some_and(char::is_alphabetic) {
            ui.label("Error: First letter must be alphabetic");
        }
        else {
            return true;
        };
        false
    }

    pub fn simulation_ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, use_imperial: bool) {
        // Simulation stuff
        self.evaluator_context.clear_variables();

        for input_value in self.input_values.iter_mut() {
            ui.label("ID:");
            ui.text_edit_singleline(&mut input_value.id);
            // TODO: Add a range setting and unit selection
            ui::length_slider(ui, &mut input_value.value, use_imperial, 0.0..=10.0, &length::meter, &length::foot);

            if evalexpr::Context::get_value(&self.evaluator_context, &input_value.id).is_some() {
                ui.label("Error: Identifier already used");
            }
            else if ChuteDesigner::validate_identifier(&input_value.id, ui) {
                self.evaluator_context.set_value(input_value.id.clone(), evalexpr::Value::Float(input_value.value)).unwrap_or_else(|_| {
                    ui.label("Unable to save value...");
                })
            }
            ui.separator();
        }

        ui.label("PARAMETERS:");

        // Similar for parameters
        for parameter in self.parametric_expressions.iter_mut() {
            ui.label("ID:");
            ui.text_edit_singleline(&mut parameter.id);
            ui.text_edit_singleline(&mut parameter.expression);

            if evalexpr::Context::get_value(&self.evaluator_context, &parameter.id).is_some() {
                ui.label("Error: Identifier already used");
            }
            else if ChuteDesigner::validate_identifier(&parameter.id, ui) {
                let computed = evalexpr::eval_number_with_context(&parameter.expression, &self.evaluator_context);
                if computed.is_ok() {
                    let value = computed.unwrap_or_default();
                    ui.label(format!("Result: {}", value));

                    self.evaluator_context.set_value(parameter.id.clone(), evalexpr::Value::Float(value));
                } else {
                    ui.label(format!("Error: {:?}", computed));
                }
            }



            ui.separator();
        }
    }

    pub fn experiment_ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, use_imperial: bool) {
        // For measurement data
        // E.g. input velocity/density and get CD
        ui.label("Descent rate");
        ui::length_slider(ui, &mut self.diameter, use_imperial, 0.0..=10.0, &si::velocity::meter_per_second, &si::velocity::foot_per_second);

    }
}


impl Default for ChuteDesigner {
    fn default() -> Self {
        let context = evalexpr::HashMapContext::new();
        // TODO: make math functions work without prefix

        let test_input1 = InputValue {description: "".into(), id: "input1".into(), range: 0.0..=10.0, unit: StandardUnit::MeterFoot, value: 0.0};
        let test_input2 = InputValue {description: "".into(), id: "input2".into(), range: 0.0..=10.0, unit: StandardUnit::MeterFoot, value: 0.0};

        let param1 = ParameterValue {display_unit: StandardUnit::MeterFoot, id: "param1".into(), expression: "1".into()};
        let param2 = ParameterValue {display_unit: StandardUnit::MeterFoot, id: "param2".into(), expression: "1".into()};
        let param3 = ParameterValue {display_unit: StandardUnit::MeterFoot, id: "param3".into(), expression: "1".into()};

        Self { 
            name: "Untitled Parachute".to_owned(),
            gores: 8,
            diameter: 1.0,
            instructions: vec!["Cut out fabric".to_owned()],
            fabric: FabricSelector::new(),
            use_global_seam_allowance: true,
            global_seam_allowance: 0.01,
            test_color: [0.0, 0.0, 0.0],
            input_values: vec![test_input1, test_input2],
            parametric_expressions: vec![param1, param2, param3],
            evaluator_context: context,
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
    points: Vec<na::Point2<f64>>,
    seam_allowance: f64,
}

impl Segment {
    fn new() -> Self {
        Self { points: vec![], seam_allowance: 0.0 }
    }

    fn new_with_allowance(seam_allowance: f64) -> Self {
        Self { points: vec![], seam_allowance: seam_allowance }
    }

    fn set_seam_allowance(&mut self, seam_allowance: f64) {
        self.seam_allowance = seam_allowance;
    }

    fn add_point(&mut self, point: na::Point2<f64>) {
        self.points.push(point);
    }

    fn add_point_xy(&mut self, x: f64, y: f64) {
        self.points.push(na::Point2::new(x, y));
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

    fn get_point(&self, idx: usize) -> na::Point2<f64> {
        self.points[idx].clone()
    }

    fn get_first_point(&self) -> na::Point2<f64> {
        self.points.first().unwrap().clone()
    }

    fn get_last_point(&self) -> na::Point2<f64> {
        self.points.last().unwrap().clone()
    }
}

struct PatternPiece {
    segments: Vec<Segment>,
    points: Vec<na::Point2<f64>>, // Points not including seams
    computed_points: Vec<na::Point2<f64>>, // Final points including seams
    fabric_area: f64, // Area in mm2, includes seam allowances
    chute_area: f64, // Area in mm2, not including seam allowances
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
        Self { segments: vec![], points: vec![], computed_points: vec![], fabric_area: 0.0, chute_area: 0.0, name: "pattern".into() }
    }

    fn add_segment(&mut self, seg: Segment) {
        self.segments.push(seg);
    }

    fn compute(&mut self) {
        // Compute seam allowances etc
        self.computed_points = vec![];
        self.points = vec![];
        for seg in self.segments.iter() {
            self.points.append(&mut seg.points.clone());
            self.computed_points.append(&mut seg.points.clone()); // todo: Add seam computation here
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
            println!("{:?}", point_pair);
            sum += point_pair[0].x * point_pair[1].y - point_pair[0].y * point_pair[1].x;
        }
        sum/2.0
    }
}

struct PatternPieceCollection {
    pieces: Vec<(PatternPiece, u32)>, // Pattern piece and number of each
}

impl PatternPieceCollection {
    fn new() -> Self {
        Self { pieces: vec![] }
    }

    fn get_area(&self, including_seams: bool) -> f64 {
        let mut total_area = 0.0;

        for (piece, count) in self.pieces.iter() {
            total_area += piece.get_area(including_seams) * (*count as f64);
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

#[derive(Default, Clone, PartialEq)]
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

#[derive(PartialEq, Clone)]
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


    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, use_imperial: bool) {
        ui.label("Select fabric:");
        egui::ComboBox::from_label("")
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
    use crate::chute::parachute::{Segment, PatternPiece};
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
        assert_eq!(seg.points[1], na::Point2::new(-1.0, 2.0));
        println!("{:?}", seg);
        seg.scale(2.0, 3.0);
        assert_eq!(seg.points[1], na::Point2::new(-2.0, 6.0));
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
}