//! Debug: where a loaded save diverges from the original run.
fn main() {
    let mut a = fidenza_world::build(17).unwrap();
    a.run(40);
    let save = a.save();
    let mut b = sim_core::sim::SimBuilder::new(0);
    b.add_plugin(&fidenza_world::FidenzaPlugin::default()).unwrap();
    let mut b = b.build_from_save(&save).unwrap();
    for t in 0..40 {
        let (sa, sb) = (serde_json::to_value(a.snapshot(true)).unwrap(), serde_json::to_value(b.snapshot(true)).unwrap());
        if sa != sb {
            let (ea, eb) = (sa["entities"].as_array().unwrap(), sb["entities"].as_array().unwrap());
            for (x, y) in ea.iter().zip(eb) {
                if x != y {
                    for (k, v) in x.as_object().unwrap() {
                        if y.get(k) != Some(v) {
                            println!("tick +{t} entità {} campo {k}: {v} vs {}", x["name"], y[k]);
                        }
                    }
                    return;
                }
            }
            for k in ["factions", "titles", "market"] {
                if sa[k] != sb[k] { println!("tick +{t} differisce {k}"); }
            }
            return;
        }
        a.tick();
        b.tick();
    }
    println!("nessuna differenza");
}
