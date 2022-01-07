use crate::algorithms::shortest_path::ShortestPathInfo;
use crate::{Error, ErrorKind, Graph, Node};
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fmt::Display;
use std::hash::Hash;

/**
As a graph is explored by a shortest-path algorithm the nodes at the
"fringe" of the explored part are maintained. This struct holds information
about a fringe node.
*/
struct FringeNode<T> {
    pub node_name: T,
    pub count: i32,
    pub distance: f64,
}

impl<T: Eq + Ord> Ord for FringeNode<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.distance < other.distance {
            Ordering::Less
        } else if self.distance > other.distance {
            Ordering::Greater
        } else {
            let count_ordering = self.count.cmp(&other.count);
            match count_ordering {
                Ordering::Equal => self.node_name.cmp(&other.node_name),
                _ => count_ordering,
            }
        }
    }
}

impl<T: Eq + Ord> PartialOrd for FringeNode<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Eq> PartialEq for FringeNode<T> {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
            && self.count == other.count
            && self.node_name == other.node_name
    }
}

impl<T: Eq> Eq for FringeNode<T> {}

static CONTRADICTORY_PATHS_ERROR_MESSAGE: &str = "Contradictary paths found, do some edges have negative weights?";

/**
Uses Dijkstra's algorithm to find shortest weighted paths between all pairs
of nodes.

# Arguments

* `graph`: a [Graph](../../../struct.Graph.html) instance where all edges have a weight.
* `cutoff`: Length (sum of edge weights) at which the search is stopped.
If cutoff is provided, only return paths with summed weight <= cutoff.
* `first_only`: If `true` returns the first shortest path found for each source and target,
if `false` returns all shortest paths found between sources and targets.

# Returns

A `HashMap` of `HashMaps`. The keys to the first one are the starting nodes
and the keys to the second are the target nodes. The values of the second one are `Vec`s
of the shortest paths between the starting and target nodes.

# Examples

```
use graphrs::{Edge, Graph, GraphSpecs, Node};
use graphrs::{algorithms::{shortest_path::{dijkstra}}};

let mut graph = Graph::<&str, ()>::new(GraphSpecs::directed_create_missing());
graph.add_edges(vec![
    Edge::with_weight("n1", "n2", 1.0),
    Edge::with_weight("n2", "n1", 2.0),
    Edge::with_weight("n1", "n3", 3.0),
    Edge::with_weight("n2", "n3", 1.1),
]);

let all_pairs = dijkstra::all_pairs(&graph, true, None, false);
assert_eq!(all_pairs.unwrap().get("n1").unwrap().get("n3").unwrap().distance, 2.1);
```

# References

1. E. W. Dijkstra. A note on two problems in connection with graphs. Numer. Math., 1:269–271, 1959.

*/
pub fn all_pairs<T, A>(
    graph: &Graph<T, A>,
    weighted: bool,
    cutoff: Option<f64>,
    first_only: bool,
) -> Result<HashMap<T, HashMap<T, ShortestPathInfo<T>>>, Error>
where
    T: Hash + Eq + Copy + Ord + Display + Send + Sync,
    A: Copy + Send + Sync,
{
    Ok(graph
        .get_all_nodes()
        .into_par_iter()
        .map(|node| {
            (
                node.name,
                single_source(graph, weighted, node.name, None, cutoff, first_only).unwrap(),
            )
        })
        .collect())
}

/**
Uses Dijkstra's algorithm to find shortest weighted paths from a single source node.
Unlike most implementations this returns all shortest paths of equal length rather
than just the first one found.

# Arguments

* `graph`: a [Graph](../../../struct.Graph.html) instance where all edges have a weight.
* `source`: The starting node.
* `target`: The ending node. If `None` then the shortest paths between `source` and
all other nodes will be found.
* `cutoff`: Length (sum of edge weights) at which the search is stopped.
If cutoff is provided, only return paths with summed weight <= cutoff.
* `first_only`: If `true` returns the first shortest path found for each target, if
`false` returns all shortest paths found between source and targets.

# Examples

```
use graphrs::{Edge, Graph, GraphSpecs, Node};
use graphrs::{algorithms::{shortest_path::{dijkstra}}};

let mut graph = Graph::<&str, ()>::new(GraphSpecs::directed_create_missing());
graph.add_edges(vec![
    Edge::with_weight("n1", "n2", 1.0),
    Edge::with_weight("n2", "n1", 2.0),
    Edge::with_weight("n1", "n3", 3.0),
    Edge::with_weight("n2", "n3", 1.1),
]);

let shortest_paths = dijkstra::single_source(&graph, true, "n1", Some("n3"), None, false);
assert_eq!(shortest_paths.unwrap().get("n3").unwrap().distance, 2.1);
```

# References

1. E. W. Dijkstra. A note on two problems in connection with graphs. Numer. Math., 1:269–271, 1959.
*/
pub fn single_source<T, A>(
    graph: &Graph<T, A>,
    weighted: bool,
    source: T,
    target: Option<T>,
    cutoff: Option<f64>,
    first_only: bool,
) -> Result<HashMap<T, ShortestPathInfo<T>>, Error>
where
    T: Hash + Eq + Copy + Ord + Display + Send + Sync,
    A: Copy,
{
    multi_source(graph, weighted, vec![source], target, cutoff, first_only)
}

/**
Uses Dijkstra's algorithm to find shortest weighted paths from multiple source nodes.
Unlike most implementations this returns all shortest paths of equal length rather
than just the first one found.

# Arguments

* `graph`: a [Graph](../../../struct.Graph.html) instance where all edges have a weight.
* `sources`: The starting nodes. The shortest path will be found that can start
for any of the `sources` and ends at the `target`.
* `target`: The ending node. If `None` then the shortest paths between `sources` and
all other nodes will be found.
* `cutoff`: Length (sum of edge weights) at which the search is stopped.
If cutoff is provided, only return paths with summed weight <= cutoff.
* `first_only`: If `true` returns the first shortest path found for each target, if
`false` returns all shortest paths found between sources and targets.

# Examples

```
use graphrs::{Edge, Graph, GraphSpecs, Node};
use graphrs::{algorithms::{shortest_path::{dijkstra}}};

let mut graph = Graph::<&str, ()>::new(GraphSpecs::directed_create_missing());
graph.add_edges(vec![
    Edge::with_weight("n1", "n2", 1.0),
    Edge::with_weight("n2", "n1", 2.0),
    Edge::with_weight("n1", "n3", 3.0),
    Edge::with_weight("n2", "n3", 1.1),
]);

let shortest_paths = dijkstra::multi_source(&graph, true, vec!["n1", "n2"], Some("n3"), None, false);
assert_eq!(shortest_paths.unwrap().get("n3").unwrap().distance, 1.1);
```

# References

1. E. W. Dijkstra. A note on two problems in connection with graphs. Numer. Math., 1:269–271, 1959.
*/
pub fn multi_source<T, A>(
    graph: &Graph<T, A>,
    weighted: bool,
    sources: Vec<T>,
    target: Option<T>,
    cutoff: Option<f64>,
    first_only: bool,
) -> Result<HashMap<T, ShortestPathInfo<T>>, Error>
where
    T: Hash + Eq + Copy + Ord + Display + Send + Sync,
    A: Copy,
{
    let shortest_path_infos =
        dijkstra_multisource(graph, weighted, sources, target, cutoff, first_only);
    match shortest_path_infos {
        Err(e) => Err(e),
        Ok(spis) => Ok(spis
            .into_iter()
            .filter(|(k, _v)| target.is_none() || k == &target.unwrap())
            .collect::<HashMap<T, ShortestPathInfo<T>>>()),
    }
}

/**
Uses Dijkstra's algorithm to find shortest weighted paths.
This is a private function that does all the work of finding the
shortest paths. All the public functions in this module call this one.
*/
fn dijkstra_multisource<T, A>(
    graph: &Graph<T, A>,
    weighted: bool,
    sources: Vec<T>,
    target: Option<T>,
    cutoff: Option<f64>,
    first_only: bool,
) -> Result<HashMap<T, ShortestPathInfo<T>>, Error>
where
    T: Hash + Eq + Copy + Ord + Display + Send + Sync,
    A: Copy,
{
    if weighted && !graph.edges_have_weight() {
        return Err(Error {
            kind: ErrorKind::EdgeWeightNotSpecified,
            message: "Not all edges in the graph have a weight.".to_string(),
        });
    }

    let get_cost = |u, v| match weighted {
        true => match graph.specs.multi_edges {
            false => get_cost_single(graph, u, v),
            true => get_cost_multi(graph, u, v),
        },
        false => 1.0,
    };

    let mut paths: HashMap<T, Vec<Vec<T>>> = sources.iter().map(|s| (*s, vec![vec![*s]])).collect();
    let mut dist = HashMap::<T, f64>::new();
    let mut seen = HashMap::<T, f64>::new();
    let mut fringe = BinaryHeap::new();
    let mut count = 0;

    for source in sources {
        seen.insert(source, 0.0);
        fringe.push(FringeNode {
            node_name: source,
            count: 0,
            distance: -0.0,
        });
    }

    while !fringe.is_empty() {
        let fringe_item = fringe.pop().unwrap();
        let d = -fringe_item.distance;
        let v = fringe_item.node_name;
        if dist.contains_key(&v) {
            continue;
        }
        dist.insert(v, d);
        if target.is_some() && v == target.unwrap() {
            break;
        }
        for node in get_successors_or_neighbors(graph, v) {
            let u = node.name;
            let cost = get_cost(v, u);
            let vu_dist = dist.get(&v).unwrap() + cost;
            if cutoff.is_some() && vu_dist > cutoff.unwrap() {
                continue;
            }
            if dist.contains_key(&u) {
                let u_dist = *dist.get(&u).unwrap();
                if vu_dist < u_dist {
                    return Err(get_contractory_paths_error());
                }
            } else if !seen.contains_key(&u) || vu_dist < *seen.get(&u).unwrap() {
                seen.insert(u, vu_dist);
                push_fringe_node(&mut count, &mut fringe, u, vu_dist);
                let mut new_paths_v = paths.entry(v).or_default().clone();
                new_paths_v.iter_mut().for_each(|pv| pv.push(u));
                paths.insert(u, new_paths_v);
            } else if !first_only && vu_dist == *seen.get(&u).unwrap() {
                push_fringe_node(&mut count, &mut fringe, u, vu_dist);
                add_u_to_v_paths_and_append_v_paths_to_u_paths(u, v, &mut paths);
            }
        }
    }

    Ok(get_shortest_path_infos::<T, A>(dist, paths))
}

/// Returns the `Error` object for a contradictory-paths error.
#[inline]
fn get_contractory_paths_error() -> Error {
    Error {
        kind: ErrorKind::ContradictoryPaths,
        message: CONTRADICTORY_PATHS_ERROR_MESSAGE.to_string(),
    }
}

/**
Pushes a `FringeNode` into the `fringe` `BinaryHeap`.
Increments `count`.
*/
#[inline]
fn push_fringe_node<T>(count: &mut i32, fringe: &mut BinaryHeap<FringeNode<T>>, u: T, vu_dist: f64)
where
    T: Hash + Eq + Copy + Ord + Display + Send + Sync,
{
    *count += 1;
    fringe.push(FringeNode {
        node_name: u,
        count: *count,
        distance: -vu_dist,
    });
}

/**
Adds `u` to the paths that lead to `v`, then appends all the paths that
lead to `v` to the paths that lead to `u`.
*/
#[inline]
fn add_u_to_v_paths_and_append_v_paths_to_u_paths<T>(
    u: T,
    v: T,
    paths: &mut HashMap<T, Vec<Vec<T>>>,
) where
    T: Hash + Eq + Copy + Ord + Display + Send + Sync,
{
    // add u to all paths[v], then *append* them to paths[u]
    let v_paths: Vec<Vec<T>> = paths
        .get(&v)
        .unwrap()
        .iter()
        .map(|p| {
            let mut x = p.clone();
            x.push(u);
            x
        })
        .collect();
    let u_paths = paths.get_mut(&u).unwrap();
    for v_path in v_paths {
        u_paths.push(v_path);
    }
}

/**
Returns the "cost" of a (`u`, `v`) edges when the `graph` is a multigraph.

Finds lowest weight of the (u, v) edges.
*/
fn get_cost_multi<T, A>(graph: &Graph<T, A>, u: T, v: T) -> f64
where
    T: Hash + Eq + Copy + Ord + Display + Send + Sync,
    A: Copy,
{
    let edges = graph.get_edges(u, v).unwrap();
    let weights = edges.into_iter().map(|e| e.weight);
    weights.into_iter().reduce(f64::min).unwrap()
}

/**
Returns the weight of the (u, v) edge in a `graph` that is not a multigraph.
*/
fn get_cost_single<T, A>(graph: &Graph<T, A>, u: T, v: T) -> f64
where
    T: Hash + Eq + Copy + Ord + Display + Send + Sync,
    A: Copy,
{
    let edge = graph.get_edge(u, v).unwrap();
    edge.weight
}

/**
Zips the `distances` and the `paths` together into a `HashMap` where
the keys are the names of the target nodes and the values are
`ShortestPathInfo` objects.
*/
fn get_shortest_path_infos<T, A>(
    distances: HashMap<T, f64>,
    paths: HashMap<T, Vec<Vec<T>>>,
) -> HashMap<T, ShortestPathInfo<T>>
where
    T: Hash + Eq + Copy + Ord + Display + Send + Sync,
    A: Copy,
{
    distances
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                ShortestPathInfo {
                    distance: v,
                    paths: paths.get(&k).unwrap().clone(),
                },
            )
        })
        .collect::<HashMap<T, ShortestPathInfo<T>>>()
}

/**
Returns successors of a node if the `graph` is directed.

Returns neighbors of a node if the `graph` is undirected.
*/
fn get_successors_or_neighbors<T, A>(graph: &Graph<T, A>, node_name: T) -> Vec<&Node<T, A>>
where
    T: Hash + Eq + Copy + Ord + Display + Send + Sync,
    A: Copy,
{
    match graph.specs.directed {
        true => graph.get_successor_nodes(node_name).unwrap(),
        false => graph.get_neighbor_nodes(node_name).unwrap(),
    }
}
