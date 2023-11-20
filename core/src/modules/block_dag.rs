use crate::imports::*;
// use ritehash::FxHasher64;
// use std::hash::BuildHasherDefault;
// type FxBuildHasher = BuildHasherDefault<FxHasher64>;
// type FxHashMap<K, V> = HashMap<K, V, FxBuildHasher>;
use egui_plot::{
    // Legend,
    LineStyle,
    Plot,
    PlotPoints, Polygon, uniform_grid_spacer, Line, PlotBounds,
    // Corner
};

pub struct BlockDag {
    #[allow(dead_code)]
    runtime: Runtime,
    daa_cursor: f64,
    last_daa_score : u64,
    running : bool,
    plot_bounds : PlotBounds,
    bezier : bool,
    parent_levels : usize,
    needs_repaint : Arc<AtomicBool>,
    // repaint_thread : Option<std::thread::JoinHandle<()>>,
    // repaint_channel: Channel<()>,
}

impl BlockDag {
    pub fn new(runtime: Runtime) -> Self {
        Self { 
            runtime, 
            daa_cursor : 0.0, 
            last_daa_score : 0, 
            running : false, 
            plot_bounds : PlotBounds::NOTHING, 
            bezier : true, 
            needs_repaint : Arc::new(AtomicBool::new(true)),
            parent_levels : 1,
            // repaint_thread : None, 
            // repaint_channel : Channel::unbounded(),
        }
    }
}

impl ModuleT for BlockDag {

    fn style(&self) -> ModuleStyle {
        ModuleStyle::Default
    }

    fn render(
        &mut self,
        core: &mut Core,
        _ctx: &egui::Context,
        _frame: &mut eframe::Frame,
        ui: &mut egui::Ui,
    ) {
        let theme = theme();

        ui.horizontal(|ui| {

            ui.heading("BlockDAG");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.checkbox(&mut self.bezier, "Bezier Curves");
                ui.separator();
                ui.add(
                    Slider::new(&mut self.parent_levels, 1..=50)
                        // .logarithmic(*logarithmic)
                        .clamp_to_range(true)
                        // .smart_aim(*smart_aim)
                        // .orientation(orientation)
                        .text("Parent levels")
                        .step_by(1.0)
                        // .trailing_fill(*trailing_fill),
                );
                // *value = value_i32 as f64;

            });
        });
        // ui.horizontal_rtl(|ui| ui.label("Right"));
        ui.separator();

        let current_daa_score = core.state().current_daa_score().unwrap_or_default();
        if self.last_daa_score != current_daa_score {

            if !self.running {
                self.running = true;
                self.daa_cursor = current_daa_score as f64;
            }

            self.last_daa_score = current_daa_score;
        }

        let delta = 0.025;
        let daa_diff = current_daa_score as f64 - self.daa_cursor;
        let step = daa_diff * delta;
        let step = (1.0 + step).powf(2.0) - 1.0;
        self.daa_cursor += step;
        
        let graph_width = ui.available_width();
        let graph_height = ui.available_height();
        let default_daa_min = self.daa_cursor -20.0;
        let default_daa_max = self.daa_cursor + 8.0;
        let default_daa_range = default_daa_max - default_daa_min;
        let pixels_per_daa = graph_width as f64 / default_daa_range;
        let bezier_steps = if pixels_per_daa < 2.0 { 2 } else { pixels_per_daa as usize / 3};

        let plot = Plot::new("block_dag")
            .width(graph_width)
            .height(graph_height)
            .include_x(default_daa_max)
            .include_x(default_daa_min)
            .include_y(15.)
            .include_y(-15.)
            .data_aspect(0.2)
            .y_axis_width(0)
            .show_axes([true, false])
            .show_grid(true)
            // .allow_drag([true, false])
            .allow_drag([true, true])
            .allow_scroll(true)
            .allow_double_click_reset(true)
            .x_axis_formatter(move |x, _size, _range| {
                format!("{} DAA", x.trunc().separated_string())
            })
            .x_grid_spacer(
                uniform_grid_spacer(move |input| {
                    let (start,stop) = input.bounds;
                    let d = (stop - start) / 5.;
                    let mut v = 10.;
                    while v < d {
                        v *= 2.;
                    }
                    [v,v*10.,v*100.]
                })
            )
            .label_formatter(move |name, point| {
                let PlotPoint { x, y: _ } = point;
                format!("{name}\n{} DAA", x.trunc().separated_string())
            })                        
            // .legend(Legend::default().position(Corner::RightTop))                            
            ;

        // let fill_color = Color32::LIGHT_BLUE;
        // let stroke_color = Color32::BLUE;

        let mut graph_settled = true;
        let mut lines = Vec::new();

        let daa_margin = 10;
        let daa_min = self.plot_bounds.min()[0] as u64 - daa_margin;
        let daa_max = self.plot_bounds.max()[0] as u64 + daa_margin;
        let blocks = if let Ok(mut daa_buckets) = self.runtime.block_dag_monitor_service().chain.lock() {
            daa_buckets.iter_mut().filter_map(|(daa_score,bucket)| {
                (*daa_score > daa_min || *daa_score < daa_max).then_some(bucket)
            }).flat_map(DaaBucket::render).collect::<Vec<_>>()
        } else {
            return;
        };

        let parent_levels = self.parent_levels.max(1);
        // println!("parent_levels: {} pl: {}", parent_levels,self.parent_levels);
        let block_map : AHashMap<KaspaHash,(PlotPoint,bool)> = blocks.clone().into_iter().map(|(block, plot_point,vspc, _)|(block.header.hash,(plot_point,vspc))).collect();
        let polygons = blocks.iter().map(|(block, point, current_vspc, block_settled)| {
            if !block_settled {
                graph_settled = false;
            }

            let PlotPoint { x, y } = *point;
            // let parent_levels = &block.header.parents_by_level;
            // for parent_hash in block.header.direct_parents() {
            for (level,parent_level) in block.header.parents_by_level.iter().enumerate() {
                if level >= parent_levels {
                    break;
                }
                // for parent_hash in block.header.parents_by_level.iter().flatten() {
                for parent_hash in parent_level.iter() {
                    if let Some(parent_point) = block_map.get(parent_hash) {
                        let (PlotPoint { x: parent_x, y: parent_y }, parent_vspc) = *parent_point;
                        let points = if self.bezier {
                            bezier(x,y,parent_x,parent_y,bezier_steps,0.6) 
                        } else {
                            [
                                [x,y],
                                [parent_x, parent_y],
                            ].into_iter().map(|pt|pt.into()).collect::<Vec<_>>()
                        };
                        if level == 0 && *current_vspc && parent_vspc {
                            lines.push(Line::new(PlotPoints::Owned(points)).color(theme.dagviz_vspc_connect_color).style(LineStyle::Solid).width(3.0));
                        } else {
                            lines.push(Line::new(PlotPoints::Owned(points)).color(theme.dagviz_parent_connect_color).style(LineStyle::Solid));
                        }
                    }
                }
            }

            // let d = 1.5;
            let d = 1.5;
            let points: PlotPoints = [
                [x+d*0.2, y+d],
                [x-d*0.2, y+d],
                [x-d*0.2, y-d],
                [x+d*0.2, y-d],
            ].to_vec().into();
        
            Polygon::new(points)
                .name(block.header.hash.to_string())
                .fill_color(theme.dagviz_block_fill_color)
                .stroke(Stroke::new(1.0, theme.dagviz_block_stroke_color))
                .style(LineStyle::Solid)

        }).collect::<Vec<_>>();

        let response = plot.show(ui, |plot_ui| {
            lines.into_iter().for_each(|line| {
                plot_ui.line(line);
            });
            polygons.into_iter().for_each(|polygon| {
                plot_ui.polygon(polygon);
            });
        });

        if daa_diff > 0.001 || !graph_settled {
            self.needs_repaint.store(true, Ordering::Relaxed);
            std::thread::sleep(Duration::from_millis(1000/30));
            crate::runtime::try_runtime().unwrap().request_repaint();
        } 
        // else {
        //     println!("skipping redraw...");
        // }

        self.plot_bounds = *response.transform.bounds();
        // println!("plot_bounds: {:?}", self.plot_bounds);

    }

    fn activate(&mut self, _core: &mut Core) {
        crate::runtime::runtime().block_dag_monitor_service().enable();

    //     let needs_repaint = self.needs_repaint.clone();
    //     let receiver = self.repaint_channel.receiver.clone();
    //     let thread = std::thread::Builder::new()
    //         .name("blockdag".to_string())
    //         .spawn(move || {
    //             // core_.run();
    //             while receiver.try_recv().is_err() {
    //                 if needs_repaint.load(Ordering::Relaxed) {
    //                     needs_repaint.store(false, Ordering::Relaxed);
    //                     crate::runtime::try_runtime().unwrap().request_repaint();
    //                 } else {
    //                     println!("skipping repaint...");
    //                 }
    //                 std::thread::sleep(std::time::Duration::from_millis(1000/30));
    //             }

    //         }).unwrap();
    //         self.repaint_thread.replace(thread);
    }

    fn deactivate(&mut self, _core: &mut Core) {
        crate::runtime::runtime().block_dag_monitor_service().disable();

    //     if let Some(join_handle) = self.repaint_thread.take() {
    //         self.repaint_channel.try_send(()).unwrap();
    //         join_handle.join().unwrap();
    //     }
    }
}

