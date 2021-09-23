use rand::prelude::*;

pub fn init(calling_id: u32, g: u32, system: Vec<u32>) -> Vec<u32> {
    // Initialise group to store randomly chosen processors and random generator.
    let mut group: Vec<u32> = Vec::new();
    let mut rng = rand::thread_rng();
    let num_proc = system.len();

    // Fill Gossip group until it is of size G.
    let gossip_group = loop {
        let n = rng.gen_range(0..num_proc);

        let random_id: u32 = system[n];

        if (random_id != calling_id) && (!group.contains(&random_id)) {
            group.push(random_id);
        }

        if group.len() == g as usize {
            break group;
        }
    };

    println!("Group is : {:?}", gossip_group);
    gossip_group
}
