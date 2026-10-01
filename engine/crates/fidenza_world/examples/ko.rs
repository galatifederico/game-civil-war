//! How often does the champion go down, and who knocks it out? (seeds 1-3, 9 days)
fn main() {
    for seed in 1..=3 {
        let mut sim = fidenza_world::build(seed).expect("mondo");
        sim.run(24 * 9);
        let ko: Vec<String> = sim.events().all().iter().filter(|e| e.kind == "knocked_out").map(|e| format!("{} da {:?}", e.tick, e.actor)).collect();
        println!("seed {seed}: {} volte al tappeto {:?}", ko.len(), ko);
    }
}
