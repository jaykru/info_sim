use petgraph::visit::EdgeRef;
use rand::prelude::*;
use petgraph::graph::UnGraph;
use petgraph::dot::{Dot, Config};
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Debug,Clone, Hash, PartialEq, Eq)]
struct Location {
    x: u64,
    y: u64,
}

#[derive(Clone,Debug)]
struct Person {
    id: u128,
    home: Location,
    has_car: bool,
    has_net_now: bool,
}


impl Person {
    fn new() -> Person {
        Person {
            id: rand::random::<u128>(),
            home: Location {x: rand::random::<u64>(), y: rand::random::<u64>()},
            has_car: rand::random::<bool>(),
            has_net_now: *[vec![false;1],vec![true;9]].concat().choose(&mut rand::thread_rng()).unwrap(),
        }
    }
}

impl Default for Person {
    fn default() -> Self {
        Person::new()
    }
}

#[derive(Eq,PartialEq,Hash,Clone,Debug)]
struct City {
    id: u128,
    center: Location,
    radius: u64,
}

impl City {
    fn new() -> City {
        let mut rng = rand::thread_rng();
        let d: f64 = rng.gen();

        City {
            id: rand::random::<u128>(),
            center: Location {x: rand::random::<u64>(), y: rand::random::<u64>()},
            radius: if d <= 0.5 { 100 } else { 12 }, // car owners can go ~100km a day, rest of pop can go 12 (bike)
        }
    }
}

use petgraph::graph::NodeIndex;
use itertools::Itertools;
// Transmit a message from start to dest, returning the amount of time spent in
// transit.

// Algorithm:

// 0. Initialize cur_nodes to [(start,0)]
// 1. If (dest,t) is in cur_nodes, return t. Else, go to 2.
// 2. Loop: For each (node,t) in cur_nodes, select (with bias towards nearby nodes) an
// adjacent node to pass the message to. Add that (adj, t+e) where e is the edge
// cost of (node -> adj) to cur_nodes. Go to 1

fn transmit_message(g: &UnGraph<Person,u64>, start: NodeIndex, dest: NodeIndex) -> u64 {
    let mut cur_nodes: HashMap<NodeIndex, u64> = HashMap::from_iter([(start,0)]);
    loop {

        if cur_nodes.contains_key(&dest) {
            return cur_nodes[&dest]
        }

        // advance to a random edge for each cur_node.
        // TODO: consider modifying this to replace cur_nodes with the frontier, or only keep some young cur_nodes which are still infected and can transmit the message.
        let mut frontier = Vec::new();
        for (node,t) in cur_nodes.iter() {
            let edge = g.edges(*node).choose(&mut rand::thread_rng()).unwrap();
            frontier.push((edge.target(),*t+edge.weight()));
            /* let edges_by_weight = g.edges(*node)
                                                                    .group_by(|edge| edge.weight());
            let mut edges: Vec<_> = edges_by_weight.into_iter().collect();
            edges.sort_by(|e1,e2| e1.0.cmp(e2.0))
            let edge = edges.choose(&mut rand::thread_rng()).unwrap()
            if rand::thread_rng().gen_range(1..=10) <= 8 {
                // with 80% probability, select a close edge
            } */
        }
        cur_nodes.extend(frontier);
    }
}

fn main() {
    // generate a couple of cities
    // cluster some nodes around the cities, connecting nodes to each other with lowish probability, skew towards creating cliques.   
    // nodes which share a connection will be connected themselves with probability 0.2.

    let mut worldgraph = UnGraph::<Person, u64>::default();    
    let cities: Vec<City> = (1..=10).map(|_| City::new()).collect();
    let mut city_nodes: HashMap<City, Vec<(Person,petgraph::graph::NodeIndex)>> = HashMap::new();
    
    // create person nodes for each city, connecting up random people in each city with 10% probability
    for city in &cities {
        let is_small = rand::random::<bool>();
        let num_nodes = if is_small { 10000 } else { 300000 };
        let people: Vec<Person> = (1..=num_nodes).map(|_| Person::new()).collect();
        let people_nodes : Vec<_> = people.iter()
                                          .cloned()
                                          .zip(people
                                               .iter()
                                               .map(|node|
                                                    worldgraph.add_node((*node).clone())))
                                          .collect();

        city_nodes.insert(city.clone(), people_nodes.clone());
        // println!("Formed nodes!");
        for (node1,node1_ind) in &people_nodes {
            for (node2,node2_ind) in &people_nodes {
                if node1.id != node2.id {
                    let d: f64 = rand::thread_rng().gen();
                    if d <= 0.01 {
                        worldgraph.add_edge(*node1_ind, *node2_ind, 1);
                    }
                }
            }
        }
        // println!("Connected intra-city nodes!");
        // second pass of nodes to form cliques
        for (node1, node1_idx) in &people_nodes {
            for (node2, node2_idx) in &people_nodes {
                let node1_neighbors: HashSet<petgraph::graph::NodeIndex> = HashSet::from_iter(worldgraph.neighbors(*node1_idx));
                let node2_neighbors: HashSet<petgraph::graph::NodeIndex> = HashSet::from_iter(worldgraph.neighbors(*node2_idx));
                
                // if node1 and node2 are unconnected nodes which have common neighbors,
                // connect them with probability 0.3a
                if node1.id != node2.id
                && !node1_neighbors.intersection(&node2_neighbors)
                                   .collect::<Vec<_>>()
                                   .is_empty()
                && !node1_neighbors.contains(&node2_idx)
                {
                    let d: f64 = rand::thread_rng().gen();
                    if d <= 0.10 {
                        worldgraph.add_edge(*node1_idx, *node2_idx, 1);
                    }
                }
            }
        }

        // connect internet-haver nodes together
        for (node1,node1_ind) in &people_nodes {
            for (node2,node2_ind) in &people_nodes {
                if node1.id != node2.id {
                    if node1.has_net_now && node2.has_net_now {
                        worldgraph.update_edge(*node1_ind, *node2_ind, 0);
                    }

                }
            }
        }
    }
    // connect adjacent cities together with 20% of the population (car owners)
    for (i,j) in cities[0..cities.len()-1]
        .iter()
        .zip(cities[1..cities.len()].iter()) {
            // println!("on city {:?}", i);
            for (person, idx) in &city_nodes[i] {
                if person.has_car {
                    // println!("connecting to city {:?}", j);
                    let j_entrypoint = city_nodes.get(j).unwrap()[0].1;
                    worldgraph.add_edge(*idx, j_entrypoint,3); // takes 3 hours to go between adjacent cities
                }

            }
        }
    println!("Finished wiring!");
    for _ in 1..=100 {
        let node1 = city_nodes[&cities[0]].choose(&mut rand::thread_rng()).unwrap().1;
        let node2 = city_nodes[&cities[9]].choose(&mut rand::thread_rng()).unwrap().1;
        let transit_time = transmit_message(&worldgraph, node1, node2);
        println!("Sending a message from {:?} to {:?} took {} hours", node1, node2, transit_time);
    }
    
/*     println!("{:?}", Dot::with_config(&worldgraph,&[Config::EdgeNoLabel, Config::NodeNoLabel])) */
}
