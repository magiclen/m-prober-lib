use std::path::Path;

use crate::{
    Error,
    scanner_rust::ScannerU8SliceAscii,
    utils::{parse_cpu_list, read_file, read_sysfs_string},
};

/// The memory and the processors of one NUMA node, read from the `/sys/devices/system/node/nodeN` folder. Every memory field is in bytes.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NumaNode {
    /// The number of this node, which is the `N` of `nodeN`.
    pub node:       usize,
    /// The logical processors that are local to this node.
    pub cpus:       Vec<usize>,
    /// The memory this node has in total (`MemTotal`).
    pub total:      u64,
    /// The memory of this node that is free (`MemFree`).
    pub free:       u64,
    /// The memory of this node that is in use, which is `total - free` (`MemUsed`).
    pub used:       u64,
    /// The page cache of this node (`FilePages`).
    pub file_pages: u64,
    /// The anonymous memory of this node (`AnonPages`).
    pub anon_pages: u64,
    /// The `tmpfs` and shared memory of this node (`Shmem`).
    pub shmem:      u64,
    /// The slab memory of this node (`Slab`).
    pub slab:       u64,
    /// The dirty memory of this node that is waiting to be written back (`Dirty`).
    pub dirty:      u64,
}

/// Parse the content of a node `meminfo` file, whose lines look like `Node 0 MemTotal:       65083032 kB`.
fn parse_node_meminfo(data: &[u8], node: &mut NumaNode) -> Result<(), Error> {
    let mut sc = ScannerU8SliceAscii::new(data);

    while let Some(token) = sc.next()? {
        // Every line starts with the `Node N` prefix, which carries no information the caller does not have.
        if token != b"Node" {
            continue;
        }

        let Some(_) = sc.next()? else {
            break;
        };

        let Some(label) = sc.next()? else {
            break;
        };

        let Some(value) = sc.next_u64()? else {
            break;
        };

        // Every field of this file is in kB.
        let value = value * 1024;

        match label {
            b"MemTotal:" => node.total = value,
            b"MemFree:" => node.free = value,
            b"MemUsed:" => node.used = value,
            b"FilePages:" => node.file_pages = value,
            b"AnonPages:" => node.anon_pages = value,
            b"Shmem:" => node.shmem = value,
            b"Slab:" => node.slab = value,
            b"Dirty:" => node.dirty = value,
            _ => (),
        }
    }

    Ok(())
}

/// Get the memory and the processors of a NUMA node by reading files in the `/sys/devices/system/node/nodeN` folder. A `NotFound` error is returned when the node does not exist.
///
/// ```rust,no_run
/// use mprober_lib::memory;
///
/// let numa_node = memory::get_numa_node(0).unwrap();
///
/// println!("{numa_node:#?}");
/// ```
pub fn get_numa_node(node: usize) -> Result<NumaNode, Error> {
    let path = Path::new("/sys/devices/system/node").join(format!("node{node}"));

    let mut numa_node = NumaNode {
        node,
        cpus: read_sysfs_string(path.join("cpulist"))
            .map(|list| parse_cpu_list(&list))
            .unwrap_or_default(),
        ..NumaNode::default()
    };

    parse_node_meminfo(&read_file(path.join("meminfo"), 2048)?, &mut numa_node)?;

    Ok(numa_node)
}

/// Get the memory and the processors of every online NUMA node by reading files in the `/sys/devices/system/node` folder. A machine without NUMA support reports a single node that owns all the memory, and a kernel without `CONFIG_NUMA` reports no node at all.
///
/// ```rust
/// use mprober_lib::memory;
///
/// let numa_nodes = memory::get_numa_nodes().unwrap();
///
/// println!("{numa_nodes:#?}");
/// ```
pub fn get_numa_nodes() -> Result<Vec<NumaNode>, Error> {
    let online = match read_sysfs_string("/sys/devices/system/node/online") {
        Ok(online) => online,
        // A kernel without NUMA support has no node folder at all.
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };

    let nodes = parse_cpu_list(&online);

    let mut numa_nodes = Vec::with_capacity(nodes.len());

    for node in nodes {
        // A node can be taken offline while the folder is being scanned.
        if let Ok(numa_node) = get_numa_node(node) {
            numa_nodes.push(numa_node);
        }
    }

    Ok(numa_nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NODE_MEMINFO: &[u8] = b"Node 0 MemTotal:       65083032 kB
Node 0 MemFree:        43608388 kB
Node 0 MemUsed:        21474644 kB
Node 0 SwapCached:            0 kB
Node 0 Dirty:              1728 kB
Node 0 AnonPages:       8950176 kB
Node 0 Shmem:            631916 kB
Node 0 Slab:             639484 kB
Node 0 FilePages:      11578372 kB
";

    #[test]
    fn parse() {
        let mut node = NumaNode::default();

        parse_node_meminfo(NODE_MEMINFO, &mut node).unwrap();

        assert_eq!(65083032 * 1024, node.total);
        assert_eq!(43608388 * 1024, node.free);
        assert_eq!(21474644 * 1024, node.used);
        assert_eq!(1728 * 1024, node.dirty);
        assert_eq!(8950176 * 1024, node.anon_pages);
        assert_eq!(631916 * 1024, node.shmem);
        assert_eq!(639484 * 1024, node.slab);
        assert_eq!(11578372 * 1024, node.file_pages);
    }
}
