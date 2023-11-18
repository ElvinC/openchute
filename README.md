# openchute
WIP parachute designer


# TODO:

Basic:

* Draw gores for spherical, elliptical, toroidal parachutes
* Calculate fabric surface area and weight
* Output to DXF
* Gore and cross section preview
* Save and load from JSON file format
* Simple UI
* Area, mass, drag estimation
* Custom seam allowances


Advanced:
* Multiple bands (e.g. DGB)
* Non-spherical parachutes (e.g. cross parachute)
* Works on the web
* 3D visualizer
* PDF output
* Shape generators (e.g. ringsail profile + number of sails)
* Basic shock load and descent simulation, including suspension line sizing
* Both circular and polygonal outer profile

Ideas

* egui for UI stuff. Could work on web as well...
* https://github.com/fschutt/printpdf for PDF
* https://docs.rs/dxf/latest/dxf/ for DXF writing
* https://github.com/asny/three-d for 3D stuff
* nalgebra for geometry stuff
