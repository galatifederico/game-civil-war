//! Debug helper: follows one entity tick by tick (position, task, progress).
use sim_core::prelude::*;
fn main() {
    let mut a = std::env::args().skip(1);
    let id = SimId(a.next().unwrap().parse().unwrap());
    let ticks: u64 = a.next().unwrap().parse().unwrap();
    let mut sim = fidenza_world::build(1).unwrap();
    for _ in 0..ticks {
        sim.tick();
        let e = sim.entity(id).unwrap();
        let t = sim.world.get::<sim_core::jobs::Task>(e).unwrap();
        let mv = sim.world.get::<sim_core::movement::Movement>(e).cloned().unwrap_or_default();
        println!("{:>3} pos={:?} action={:?} job={:?} speed={:.2} budget={:.2} path={} stuck={} money={:.0} fame={:.2}", sim.tick_count(), sim.world.get::<Position>(e).map(|p| (p.x, p.y)),
            t.action, t.job.as_ref().map(|j| (&j.job, j.progress, j.required, j.target)), sim_core::movement::speed_of(&sim.world, e), mv.budget, mv.path.len(), t.stuck,
            sim.world.get::<Wallet>(e).map_or(0.0, |w| w.0), sim.world.get::<Needs>(e).map_or(0.0, |n| n.get("fame")));
    }
}
