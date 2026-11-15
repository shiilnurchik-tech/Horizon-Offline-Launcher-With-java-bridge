//! Separate ticket ownership for Java and native mods sharing the same chunk streamer.
use super::Streaming;
use std::collections::HashSet;

type Chunk = (i32, i32);

#[derive(Default)]
pub(super) struct TicketOwners {
    native: HashSet<Chunk>,
    java: HashSet<Chunk>,
}

impl TicketOwners {
    fn union(&self) -> HashSet<Chunk> {
        self.native.union(&self.java).copied().collect()
    }
}

impl Streaming {
    /// Replace the Java mods' ticket set, without releasing chunks native mods still hold.
    pub fn set_ticketed(&mut self, chunks: impl IntoIterator<Item = Chunk>) {
        // Generation expands ticket neighborhoods; bound coordinates before arithmetic.
        self.mod_ticket_owners.java = chunks.into_iter().filter(|(x, z)| {
            (-1_875_000..1_875_000).contains(x) && (-1_875_000..1_875_000).contains(z)
        }).collect();
        self.refresh_mod_tickets();
    }

    /// Change the native mods' aggregate ticket for a chunk.
    pub fn set_native_ticket(&mut self, chunk: Chunk, held: bool) {
        let changed = if held {
            self.mod_ticket_owners.native.insert(chunk);
            if self.mod_forced.insert(chunk) {
                self.rank_forced(chunk.0, chunk.1, chunk.0, chunk.1);
                true
            } else {
                false
            }
        } else {
            self.mod_ticket_owners.native.remove(&chunk);
            !self.mod_ticket_owners.java.contains(&chunk) && self.mod_forced.remove(&chunk)
        };
        if changed {
            self.generation_scan.exhausted = false;
        }
    }

    fn refresh_mod_tickets(&mut self) {
        let union = self.mod_ticket_owners.union();
        let mut new: Vec<_> = union.difference(&self.mod_forced).copied().collect();
        new.sort_unstable();
        let changed = self.mod_forced != union;
        self.mod_forced = union;
        for (x, z) in new {
            self.rank_forced(x, z, x, z);
        }
        if changed {
            self.generation_scan.exhausted = false;
        }
    }

    /// The Java mods' tickets, separate from native mods' and /forceload.
    pub fn ticketed(&self) -> &HashSet<Chunk> {
        &self.mod_ticket_owners.java
    }

    /// All chunks kept by /forceload or by any mod, without duplicates.
    pub fn kept_chunks(&self) -> impl Iterator<Item = Chunk> + '_ {
        self.forced.union(&self.mod_forced).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_owners_keep_shared_chunk_until_both_release() {
        let mut owners = TicketOwners::default();
        owners.native.insert((1, 2));
        owners.java.insert((1, 2));
        owners.java.insert((3, 4));
        owners.java.remove(&(1, 2));
        assert_eq!(owners.union(), HashSet::from([(1, 2), (3, 4)]));
        owners.java.insert((1, 2));
        owners.native.remove(&(1, 2));
        assert!(owners.union().contains(&(1, 2)));
        owners.java.remove(&(1, 2));
        assert!(!owners.union().contains(&(1, 2)));
    }
}
