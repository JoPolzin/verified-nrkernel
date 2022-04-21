#[allow(unused_imports)]
use builtin::*;
use builtin_macros::*;
#[macro_use]
use crate::pervasive::*;
use seq::*;
use map::*;
#[allow(unused_imports)]
use set::*;
#[allow(unused_imports)]
use crate::{seq, seq_insert_rec, map, map_insert_rec};
#[allow(unused_imports)]
use result::{*, Result::*};

pub struct MemRegion { pub base: nat, pub size: nat }

// TODO use VAddr, PAddr

#[spec]
pub fn strictly_decreasing(s: Seq<nat>) -> bool {
    forall(|i: nat, j: nat| i < j && j < s.len() >>= s.index(i) > s.index(j))
}

// page_size, next_sizes
// 2**40    , [ 2 ** 30, 2 ** 20 ]
// 2**30    , [ 2 ** 20 ]
// 2**20    , [ ]

// [es0 # es1 , es2 , es3 ] // entry_size
// [1T  # 1G  , 1M  , 1K  ] // pages mapped at this level are this size <--

// [n0  # n1  , n2  , n3  ] // number_of_entries
// [1   # 1024, 1024, 1024] 

// es1 == es0 / n1 -- n1 * es1 == es0
// es2 == es1 / n2 -- n2 * es2 == es1
// es3 == es2 / n3 -- n3 * es3 == es2

// [es0  #  es1 , es2 , es3 , es4 ] // entry_size
// [256T #  512G, 1G  , 2M  , 4K  ]
// [n0   #  n1  , n2  , n3  , n4  ] // number_of_entries
// [     #  512 , 512 , 512 , 512 ]
// [     #  9   , 9   , 9   , 9   , 12  ]

pub struct ArchLayer {
    /// Address space size mapped by a single entry at this layer
    pub entry_size: nat,
    /// Number of entries of at this layer
    pub num_entries: nat,
}

#[spec]
pub struct Arch {
    pub layers: Seq<ArchLayer>,
    // [512G, 1G  , 2M  , 4K  ] 
    // [512 , 512 , 512 , 512 ]
}

impl Arch {
    #[spec]
    pub fn inv(&self) -> bool {
        forall(|i:nat| with_triggers!([self.layers.index(i).entry_size], [self.layers.index(i).num_entries] => i < self.layers.len() >>= (
            true
            && self.layers.index(i).entry_size > 0
            && self.layers.index(i).num_entries > 0
            && ((i + 1 < self.layers.len()) >>=
                self.layers.index(i).entry_size == self.layers.index(i as int + 1).entry_size * self.layers.index(i as int + 1).num_entries))))
    }

    #[spec] pub fn contains_entry_size(&self, entry_size: nat) -> bool {
        exists(|i: nat| #[trigger] self.layers.index(i).entry_size == entry_size)
    }
}

#[proof]
fn arch_inv_test() {
    let x86 = Arch {
        layers: seq![
            ArchLayer { entry_size: 512 * 1024 * 1024 * 1024, num_entries: 512 },
            ArchLayer { entry_size: 1 * 1024 * 1024 * 1024, num_entries: 512 },
            ArchLayer { entry_size: 2 * 1024 * 1024, num_entries: 512 },
            ArchLayer { entry_size: 4 * 1024, num_entries: 512 },
        ],
    };
    assert(x86.inv());
    assert(x86.layers.index(3).entry_size == 4096);
    assert(x86.contains_entry_size(4096));
}

#[proof]
pub struct PageTableContents {
    pub map: Map<nat /* VAddr */, MemRegion>,
    pub arch: Arch,
}

#[spec]
pub fn aligned(addr: nat, size: nat) -> bool {
    addr % size == 0
}

// TODO: overlap probably shouldn't be defined in terms of MemRegion, since it's never actually
// used that way. We always check overlap of the virtual address space.
#[spec]
pub fn overlap(region1: MemRegion, region2: MemRegion) -> bool {
    if region1.base <= region2.base {
        region2.base < region1.base + region1.size
    } else {
        region1.base < region2.base + region2.size
    }
}

fn overlap_sanity_check() {
    assert(overlap(
            MemRegion { base: 0, size: 4096 },
            MemRegion { base: 0, size: 4096 }));
    assert(overlap(
            MemRegion { base: 0, size: 8192 },
            MemRegion { base: 0, size: 4096 }));
    assert(overlap(
            MemRegion { base: 0, size: 4096 },
            MemRegion { base: 0, size: 8192 }));
    assert(overlap(
            MemRegion { base: 0, size: 8192 },
            MemRegion { base: 4096, size: 4096 }));
    assert(!overlap(
            MemRegion { base: 4096, size: 8192 },
            MemRegion { base: 0, size: 4096 }));
    assert(!overlap(
            MemRegion { base: 0, size: 4096 },
            MemRegion { base: 8192, size: 16384 }));
}

impl PageTableContents {
    #[spec]
    pub fn ext_equal(self, pt2: PageTableContents) -> bool {
        self.map.ext_equal(pt2.map)
    }

    #[spec]
    pub fn inv(&self) -> bool {
        true
        && self.arch.inv()
        && forall(|va: nat| with_triggers!([self.map.index(va).size],[self.map.index(va).base] => self.map.dom().contains(va) >>=
                  (aligned(va, self.map.index(va).size)
                   && aligned(self.map.index(va).base, self.map.index(va).size))))
        // && forall(|b1: nat, b2: nat| // TODO verus the default triggers were bad
        //     (self.map.dom().contains(b1) && self.map.dom().contains(b2)) >>= ((b1 == b2) || !overlap(
        //         MemRegion { base: b1, size: (#[trigger] self.map.index(b1)).size },
        //         MemRegion { base: b2, size: (#[trigger] self.map.index(b2)).size }
        //     )))
        && forall(|b1: nat, b2: nat| // TODO verus the default triggers were bad
            with_triggers!([self.map.index(b1), self.map.index(b2)],
                           [self.map.dom().contains(b1), self.map.dom().contains(b2)] =>
                           (self.map.dom().contains(b1) && self.map.dom().contains(b2)) >>= ((b1 == b2) || !overlap(
                MemRegion { base: b1, size: self.map.index(b1).size },
                MemRegion { base: b2, size: self.map.index(b2).size }
            ))))
    }

    #[spec] pub fn accepted_mapping(self, base: nat, frame: MemRegion) -> bool {
        true
        && aligned(base, frame.size)
        && aligned(frame.base, frame.size)
        && self.arch.contains_entry_size(frame.size)
    }

    #[spec] pub fn valid_mapping(self, base: nat, frame: MemRegion) -> bool {
        forall(|b: nat| #[auto_trigger] self.map.dom().contains(b) >>= !overlap(
                MemRegion { base: base, size: frame.size },
                MemRegion { base: b, size: self.map.index(b).size }
                ))
    }

    /// Maps the given `frame` at `base` in the address space
    #[spec] pub fn map_frame(self, base: nat, frame: MemRegion) -> Result<PageTableContents,()> {
        if self.accepted_mapping(base, frame) {
            if self.valid_mapping(base, frame) {
                Ok(PageTableContents {
                    map: self.map.insert(base, frame),
                    ..self
                })
            } else {
                Err(())
            }
        } else {
            arbitrary()
        }
    }

    // don't think this is actually necessary for anything?
    #[proof] fn map_frame_maps_valid(#[spec] self, base: nat, frame: MemRegion) {
        requires([
            self.inv(),
            self.accepted_mapping(base, frame),
            self.valid_mapping(base, frame),
        ]);
        ensures([
            self.map_frame(base, frame).is_Ok(),
        ]);
    }

    #[proof] fn map_frame_preserves_inv(#[spec] self, base: nat, frame: MemRegion) {
        requires([
            self.inv(),
            self.accepted_mapping(base, frame),
            self.map_frame(base, frame).is_Ok(),
        ]);
        ensures([
            self.map_frame(base, frame).get_Ok_0().inv(),
        ]);
    }

    // #[proof] #[verifier(non_linear)]
    // pub fn lemma_overlap_aligned_equal_size_implies_equal_base(va1: nat, va2: nat, size: nat) {
    //     requires([
    //         aligned(va1, size),
    //         aligned(va2, size),
    //         size > 0,
    //         overlap(
    //             MemRegion { base: va1, size: size },
    //             MemRegion { base: va2, size: size }),
    //     ]);
    //     ensures(va1 == va2);
    // }

    #[proof]
    pub fn lemma_overlap_IMP_equal_base(self, va1: nat, base: nat, size: nat) {
        requires([
                 self.inv(),
                 self.map.dom().contains(va1),
                 aligned(base, size),
                 size == self.map.index(va1).size,
                 size > 0, // TODO: this should probably be self.arch.layer_sizes.contains(size), along with 0 not being a valid size in the invariant
                 overlap(
                     MemRegion { base: va1, size: self.map.index(va1).size },
                     MemRegion { base: base, size: size }),
        ]);
        ensures(va1 == base);

        if va1 <= base {
            // assert(va1 + va1_size <= base);
            if va1 < base {
                assert(va1 < base);
                assert(base < va1 + size);
                assert(base % size == 0);
                assert(va1 % size == 0);
                // TODO: same as below
                assume(false);
                assert(va1 == base);
            } else { }
        } else {
            assert(base < va1);
            assert(va1 < base + size);
            assert(va1 % size == 0);
            assert(base % size == 0);
            // assert(va1 % size == va1 - base);

            // base    size
            // |-------|
            //     |-------|
            //     va1     size
            // TODO: need nonlinear reasoning? (isabelle sledgehammer can prove this)
            assume(false);
            assert(va1 == base);
        }
    }

    // predicate (function -> bool)
    // #[spec] pub fn step_map_frame(&self /* s */, post: &PageTableContents /* s' */, base:nat, frame: MemRegion) -> bool {
    //     post == self.map_frame(base, frame)
    // }

    // TODO later /// Changes the mapping permissions of the region containing `vaddr` to `rights`.
    // TODO later fn adjust(self, vaddr: nat) -> Result<(VAddr, usize), KError>;

    /// Given a virtual address `vaddr` it returns the corresponding `PAddr`
    /// and access rights or an error in case no mapping is found.
    // #[spec] fn resolve(self, vaddr: nat) -> MemRegion {
    #[spec] fn resolve(self, vaddr: nat) -> nat {
        if exists(|n:nat|
                  self.map.dom().contains(n) &&
                  n <= vaddr && vaddr < n + (#[trigger] self.map.index(n)).size) {
            let n = choose(|n:nat|
                           self.map.dom().contains(n) &&
                           n <= vaddr && vaddr < n + (#[trigger] self.map.index(n)).size);
            let offset = vaddr - n;
            self.map.index(n).base + offset
        } else {
            arbitrary()
        }
    }

    /// Removes the frame from the address space that contains `base`.
    #[spec] fn unmap(self, base: nat) -> PageTableContents {
        if self.map.dom().contains(base) {
            PageTableContents {
                map: self.map.remove(base),
                ..self
            }
        } else {
            arbitrary()
        }
    }

    #[proof] fn unmap_preserves_inv(self, base: nat) {
        requires([
            self.inv(),
            self.map.dom().contains(base),
        ]);
        ensures([
            self.unmap(base).inv()
        ]);
    }
}



// Second refinement layer

#[proof] #[is_variant]
pub enum NodeEntry {
    Directory(Directory),
    Page(MemRegion),
    Empty(),
}

#[proof]
pub struct Directory {
    pub entries: Seq<NodeEntry>,
    pub layer: nat,       // index into layer_sizes
    pub base_vaddr: nat,
    pub arch: Arch,
}
// 
// // Layer 0: 425 Directory ->
// // Layer 1: 47  Directory ->
// // Layer 2: 5   Page (1K)
// 
// // Layer 1: 46  Directory -> (1M)
// // Layer 2: 1024 Pages
// 
// // Layer 0: 1024 Directories (1T)
// // Layer 1: 1024 Directories (1G)
// // Layer 2: 1024 Pages

impl Directory {

    #[spec]
    pub fn well_formed(&self) -> bool {
        true
        && self.arch.inv()
        && aligned(self.base_vaddr, self.entry_size() * self.num_entries())
        && self.layer < self.arch.layers.len()
        && self.entries.len() == self.num_entries()
    }

    #[spec]
    pub fn arch_layer(&self) -> ArchLayer {
        recommends(self.well_formed());
        self.arch.layers.index(self.layer)
    }

    #[spec]
    pub fn entry_size(&self) -> nat {
        recommends(self.layer < self.arch.layers.len());
        self.arch.layers.index(self.layer).entry_size
    }

    #[spec]
    pub fn num_entries(&self) -> nat { // number of entries
        recommends(self.layer < self.arch.layers.len());
        self.arch.layers.index(self.layer).num_entries
    }

    #[spec(checked)]
    pub fn pages_match_entry_size(&self) -> bool {
        recommends(self.well_formed());
        forall(|i: nat| (i < self.entries.len() && self.entries.index(i).is_Page())
               >>= (#[trigger] self.entries.index(i)).get_Page_0().size == self.entry_size())
    }

    #[spec(checked)]
    pub fn directories_are_in_next_layer(&self) -> bool {
        recommends(self.well_formed());
        forall(|i: nat| (i < self.entries.len() && self.entries.index(i).is_Directory())
               >>= {
                    let directory = (#[trigger] self.entries.index(i)).get_Directory_0();
                    true
                    && directory.layer == self.layer + 1
                    && directory.base_vaddr == self.base_vaddr + i * self.entry_size()
                })
    }

    #[spec(checked)]
    pub fn directories_obey_invariant(&self) -> bool {
        decreases((self.arch.layers.len() - self.layer, 0));
        recommends(self.well_formed() && self.directories_are_in_next_layer() && self.directories_match_arch());

        if self.well_formed() && self.directories_are_in_next_layer() && self.directories_match_arch() {
            forall(|i: nat| (i < self.entries.len() && self.entries.index(i).is_Directory())
                   >>= (#[trigger] self.entries.index(i)).get_Directory_0().inv())
        } else {
            arbitrary()
        }
    }

    #[spec(checked)]
    pub fn directories_match_arch(&self) -> bool {
        forall(|i: nat| (i < self.entries.len() && self.entries.index(i).is_Directory())
               >>= equal((#[trigger] self.entries.index(i)).get_Directory_0().arch, self.arch))
    }

    #[spec(checked)]
    pub fn frames_aligned(&self) -> bool {
        recommends(self.well_formed());
        forall(|i: nat| i < self.entries.len() && self.entries.index(i).is_Page() >>=
                  aligned((#[trigger] self.entries.index(i)).get_Page_0().base, self.entry_size()))
    }

    #[spec(checked)]
    pub fn inv(&self) -> bool {
        decreases(self.arch.layers.len() - self.layer);

        self.well_formed()
        && true
        && self.pages_match_entry_size()
        && self.directories_are_in_next_layer()
        && self.directories_match_arch()
        && self.directories_obey_invariant()
        && self.frames_aligned()
    }

    // forall self :: self.directories_obey_invariant()

    #[spec(checked)]
    pub fn interp(self) -> PageTableContents {
        self.interp_aux(0)
    }

    #[spec(checked)]
    pub fn interp_aux(self, i: nat) -> PageTableContents {
        decreases((self.arch.layers.len() - self.layer, self.num_entries() - i));
        // decreases_by(Self::check_interp_aux);

        if self.inv() {
            if i >= self.entries.len() {
                PageTableContents {
                    map: map![],
                    arch: self.arch,
                }
            } else { // i < self.entries.len()
                     // TODO: using map also impossible (like fold)?
                     // let maps = self.entries.map(|i:nat| );
                let rem = self.interp_aux(i + 1).map;
                PageTableContents {
                    map: match self.entries.index(i) {
                        NodeEntry::Page(p)      => rem.insert(self.base_vaddr + i * self.entry_size(), p),
                        NodeEntry::Directory(d) => rem.union_prefer_right(d.interp_aux(0).map),
                        NodeEntry::Empty()      => rem,
                    },
                    arch: self.arch,
                }
            }
        } else {
            arbitrary()
        }
    }

//     #[proof]
//     fn x2(self) {
//         requires(self.inv());
//         // ensures(forall(|offset:nat|
//         //                self.interp().map.contains(offset)
//         //                >>= self.base_vaddr <= self.interp().map.index(offset).base
//         //                     && self.interp().map.index(offset).base + self.interp().map.index(offset).size <= self.base_vaddr + self.layer_size() * self.entry_size()));
//     }
//
    #[proof]
    fn inv_implies_interp_aux_inv(self, i: nat) {
        decreases((self.arch.layers.len() - self.layer, self.num_entries() - i));
        requires(self.inv());
        ensures([
            self.interp_aux(i).inv(),
            forall(|va: nat| #[trigger] self.interp_aux(i).map.dom().contains(va) >>= va >= self.base_vaddr + i * self.entry_size()),
            forall(|va: nat| #[trigger] self.interp_aux(i).map.dom().contains(va) >>= va <  self.base_vaddr + self.num_entries() * self.entry_size()),
        ]);

        let interp = self.interp_aux(i);

        assert(self.directories_obey_invariant());
        assert_forall_by(|i: nat| {
            requires(i < self.entries.len() && self.entries.index(i).is_Directory());
            ensures((#[trigger] self.entries.index(i)).get_Directory_0().interp_aux(0).inv());
            assert(self.entries.index(i).get_Directory_0().inv());
            self.entries.index(i).get_Directory_0().inv_implies_interp_aux_inv(0);
        });
        assert(interp.arch.inv());
        if i >= self.entries.len() {
        } else {
            self.inv_implies_interp_aux_inv(i + 1);
            if self.entries.index(i).is_Page() {
                let rem = self.interp_aux(i + 1).map;
                // TODO: verus bug?
                assume(equal(self.interp_aux(i).map, rem.insert(self.base_vaddr + i * self.entry_size(), self.entries.index(i).get_Page_0())));
                assert(equal(self.interp_aux(i).map.dom(), rem.dom().insert(self.base_vaddr + i * self.entry_size())));
            }
        }
        assert_forall_by(|va: nat| {
            requires(interp.map.dom().contains(va));
            ensures(true
                && aligned(va, (#[trigger] interp.map.index(va)).size)
                && aligned(interp.map.index(va).base, interp.map.index(va).size)
            );

            if i >= self.entries.len() {
            } else {
                let j = i + 1;
                // self.inv_implies_interp_aux_inv(i + 1);
                self.inv_implies_interp_aux_inv(j);
                assume(forall(|va: nat| #[trigger] self.interp_aux(j).map.dom().contains(va) >>= va >= self.base_vaddr + j * self.entry_size())); // TODO: verus bug?
                assume(forall(|va: nat| #[trigger] self.interp_aux(i + 1).map.dom().contains(va) >>= va >= self.base_vaddr + (i + 1) * self.entry_size()));
                if self.entries.index(i).is_Page() {
                    assert(equal(self.interp_aux(i).map.dom(), self.interp_aux(i + 1).map.dom().insert(self.base_vaddr + i * self.entry_size())));
                    if va < self.base_vaddr + i * self.entry_size() {
                        assert(va >= self.base_vaddr + (i + 1) * self.entry_size());
                        // TODO: verus bug?
                        let es: nat = self.entry_size();
                        crate::lib::mul_distributive(i, es);
                        assume((i + 1) * es == i * es + es); // TODO verus ???
                        assume((i + 1) * self.entry_size() == i * self.entry_size() + self.entry_size()); // TODO verus ???
                        assert(va >= self.base_vaddr + i * self.entry_size() + self.entry_size());
                        assert(false);
                    } else if va == self.base_vaddr + i * self.entry_size() {
                        assert(interp.map.index(va).size == self.entry_size());
                        assert(self.well_formed());
                        assume(aligned(self.base_vaddr, self.entry_size() * self.num_entries())); // TODO verus bug
                        assume(aligned(self.base_vaddr, self.entry_size())); // TODO verus nonlinear
                        assume((i * self.entry_size()) % self.entry_size() == 0); // TODO verus nonlinear
                        assert(aligned(i * self.entry_size(), self.entry_size()));
                        assume(aligned(self.base_vaddr + i * self.entry_size(), self.entry_size())); // TODO verus nonlinear
                        assert(aligned(va, interp.map.index(va).size));
                    } else {
                        assert(aligned(va, interp.map.index(va).size));
                    }
                    assert(aligned(interp.map.index(va).base, interp.map.index(va).size));
                }
            }
        });
        assert_forall_by(|b1: nat, b2: nat| {
            requires(interp.map.dom().contains(b1) && interp.map.dom().contains(b2) && b1 != b2);
            ensures(!overlap(
                MemRegion { base: b1, size: interp.map.index(b1).size },
                MemRegion { base: b2, size: interp.map.index(b2).size }
            ));

            if i >= self.entries.len() {
            } else {
                self.inv_implies_interp_aux_inv(i + 1);
                assume(forall(|va: nat| #[trigger] self.interp_aux(i + 1).map.dom().contains(va) >>= va >= self.base_vaddr + (i + 1) * self.entry_size()));
                assume(forall(|va: nat| #[trigger] self.interp_aux(i + 1).map.dom().contains(va) >>= va <  self.base_vaddr + self.num_entries() * self.entry_size())); // TODO verus bug
                let (c1, c2) = if b1 < b2 {
                    (b1, b2)
                } else {
                    (b2, b1)
                };
                assert(c1 < c2);
                match self.entries.index(i) {
                    NodeEntry::Page(p) => {
                        let new_va = self.base_vaddr + i * self.entry_size();
                        assert_by(!overlap(
                                MemRegion { base: c1, size: interp.map.index(c1).size },
                                MemRegion { base: c2, size: interp.map.index(c2).size }
                                ), {
                            if c1 != new_va && c2 != new_va {
                                assert(!overlap( // TODO is this really necessary?
                                        MemRegion { base: c1, size: interp.map.index(c1).size },
                                        MemRegion { base: c2, size: interp.map.index(c2).size }
                                        ));
                            } else if c1 == new_va {
                                assert(equal(self.interp_aux(i).map.dom(), self.interp_aux(i + 1).map.dom().insert(new_va)));
                                assert(self.interp_aux(i + 1).map.dom().contains(c2));
                                assert(c2 >= self.base_vaddr + (i + 1) * self.entry_size());
                                crate::lib::mul_distributive(i, self.entry_size());
                                assume(i * self.entry_size() + self.entry_size() == (i + 1) * self.entry_size()); // TODO verus bug
                                assert(self.base_vaddr + i * self.entry_size() + self.entry_size() ==
                                       self.base_vaddr + (i + 1) * self.entry_size());
                                assert(c1 + self.entry_size() == self.base_vaddr + (i + 1) * self.entry_size());
                                assert(!overlap( // TODO is this really necessary?
                                        MemRegion { base: c1, size: interp.map.index(c1).size },
                                        MemRegion { base: c2, size: interp.map.index(c2).size }
                                        ));
                            } else {
                                assert(c2 == new_va);
                                assert(self.interp_aux(i + 1).map.dom().contains(c1));
                                assert(c1 >= self.base_vaddr + (i + 1) * self.entry_size());
                                assert(c2 == self.base_vaddr + i * self.entry_size());
                                assume(c1 >= c2); // TODO verus nonlinear
                                assert(false);
                            }
                        });
                    },
                    NodeEntry::Directory(d) => {
                        assert_by(!overlap(
                                MemRegion { base: c1, size: interp.map.index(c1).size },
                                MemRegion { base: c2, size: interp.map.index(c2).size }
                                ), {
                            d.inv_implies_interp_aux_inv(0);
                            assume(forall(|va: nat| #[trigger] d.interp_aux(0).map.dom().contains(va) >>= va <  d.base_vaddr + d.num_entries() * d.entry_size())); // TODO verus bug
                            assert(forall(|va: nat| #[trigger] d.interp_aux(0).map.dom().contains(va) >>= va >= d.base_vaddr)); // TODO verus bug
                            assert(forall(|va: nat| #[trigger] self.interp_aux(i + 1).map.dom().contains(va) >>= va >= self.base_vaddr + (i + 1) * self.entry_size()));
                            assert(forall(|va: nat| #[trigger] self.interp_aux(i + 1).map.dom().contains(va) >>= va <  self.base_vaddr + self.num_entries() * self.entry_size())); // TODO verus bug
                            assert(self.directories_are_in_next_layer());
                            // TODO: this should be trivial consequence of directories_are_in_next_layer
                            assume(forall(|i: nat| (i < self.entries.len() && self.entries.index(i).is_Directory()) >>= {
                                let directory = (self.entries.index(i)).get_Directory_0();
                                true
                                    && directory.layer == self.layer + 1
                                    && directory.base_vaddr == self.base_vaddr + i * self.entry_size()
                            }));
                            assert(i < self.entries.len() && self.entries.index(i).is_Directory());
                            // TODO: trivial consequence of the two previous assertions
                            assume(self.entries.index(i).get_Directory_0().base_vaddr == self.base_vaddr + i * self.entry_size());
                            assert(d.base_vaddr == self.base_vaddr + i * self.entry_size());
                            // FIXME: I thought we already added this to the invariant somewhere
                            // but I can't find it?
                            assume(d.num_entries() * d.entry_size() == self.entry_size());
                            let i1_interp = self.interp_aux(i + 1).map;
                            let d_interp = d.interp_aux(0).map;
                            if i1_interp.dom().contains(c1) && i1_interp.dom().contains(c2) {
                                assert(self.interp_aux(i + 1).inv());
                                assert(!overlap(
                                        MemRegion { base: c1, size: i1_interp.index(c1).size },
                                        MemRegion { base: c2, size: i1_interp.index(c2).size }
                                        ));
                                assert_by(true
                                          && !d_interp.dom().contains(c1)
                                          && !d_interp.dom().contains(c2), {
                                              if d_interp.dom().contains(c1) {
                                                  assert(c1 < d.base_vaddr + d.num_entries() * d.entry_size());
                                                  assert(c1 < self.base_vaddr + i * self.entry_size() + d.num_entries() * d.entry_size());
                                                  // TODO: consequence of invariant:
                                                  assume(c1 < self.base_vaddr + i * self.entry_size() + self.entry_size());
                                                  assume(c1 < self.base_vaddr + (i + 1) * self.entry_size());
                                                  assert(c1 >= self.base_vaddr + (i + 1) * self.entry_size());
                                                  assert(false);
                                              } else {
                                                  if d_interp.dom().contains(c2) {
                                                      assert(c2 < d.base_vaddr + d.num_entries() * d.entry_size());
                                                      assert(c2 < self.base_vaddr + i * self.entry_size() + d.num_entries() * d.entry_size());
                                                      assume(c2 < self.base_vaddr + i * self.entry_size() + self.entry_size());
                                                      assume(c2 < self.base_vaddr + (i + 1) * self.entry_size());
                                                      assert(c1 >= self.base_vaddr + (i + 1) * self.entry_size());
                                                      assert(c2 < c1);
                                                  }
                                              }
                                          });
                                assert(equal(interp.map.index(c1), i1_interp.index(c1)));
                                assert(equal(interp.map.index(c2), i1_interp.index(c2)));
                                assert(!overlap(
                                        MemRegion { base: c1, size: interp.map.index(c1).size },
                                        MemRegion { base: c2, size: interp.map.index(c2).size }
                                        ));
                            } else if d_interp.dom().contains(c1) && d_interp.dom().contains(c2) {
                                assert(!overlap(
                                        MemRegion { base: c1, size: interp.map.index(c1).size },
                                        MemRegion { base: c2, size: interp.map.index(c2).size }
                                        ));
                            } else if d_interp.dom().contains(c1) && i1_interp.dom().contains(c2) {
                                // assert(c1 < c2);
                                assert(c2 < self.base_vaddr + self.num_entries() * self.entry_size());
                                // assert(c2 >= self.base_vaddr + (i + 1) * self.entry_size());
                                assert(c1 >= d.base_vaddr);
                                assert(c1 >= self.base_vaddr + i * self.entry_size());
                                // need to show !overlap here, no contradiction
                                assert(!overlap(
                                        MemRegion { base: c1, size: interp.map.index(c1).size },
                                        MemRegion { base: c2, size: interp.map.index(c2).size }
                                        ));
                                assume(false);
                            } else {
                                assert(d_interp.dom().contains(c2) && i1_interp.dom().contains(c1));
                                assert(c1 >= self.base_vaddr + (i + 1) * self.entry_size());
                                assert(c2 <  d.base_vaddr + d.num_entries() * d.entry_size());
                                assert(c2 <  (self.base_vaddr + i * self.entry_size()) + d.num_entries() * d.entry_size());
                                assert(c2 <  (self.base_vaddr + i * self.entry_size()) + self.entry_size());
                                // TODO: nonlinear
                                assume(c2 <  self.base_vaddr + (i + 1) * self.entry_size());
                                assert(c2 < c1);
                                assert(false);
                            }
                        });
                    },
                    NodeEntry::Empty() => (),
                }
                assert(!overlap( // TODO is this really necessary?
                        MemRegion { base: c1, size: interp.map.index(c1).size },
                        MemRegion { base: c2, size: interp.map.index(c2).size }
                        ));
            }
        });
        assume(forall(|va: nat| self.interp_aux(i).map.dom().contains(va) >>= va >= self.base_vaddr + i * self.entry_size()));
        assume(forall(|va: nat| #[trigger] self.interp_aux(i).map.dom().contains(va) >>= va <  self.base_vaddr + self.num_entries() * self.entry_size()));
    }

}
// 
//     #[proof] #[verifier(decreases_by)]
//     fn check_interp_aux(self, i: nat) {
//         assume(false);
//         // if i >= self.entries.len() {
//         //     assume(false);
//         // } else {
//         //     // TODO
//         //     assume(false);
//         // }
//     }
// 
//     // #[proof]
//     // fn x0(i: nat, m1: Map<nat,nat>, m2: Map<nat,nat>) {
//     //     requires(m1.dom().contains(i));
//     //     ensures([
//     //             m1.union_prefer_right(m2).dom().contains(i),
//     //             m2.union_prefer_right(m1).dom().contains(i)
//     //     ]);
//     // }
// 
//     #[proof]
//     fn x1(self, j: nat, i: nat) {
//         decreases(self.entries.len() - i);
//         requires([
//                  // self.entry_size() > 0,
//                  self.inv(),
//                  // self.entries.index(i).is_Page()
//                  j >= i,
//                  j < self.entries.len(),
//                  self.entries.index(j).is_Page(),
//         ]);
//         ensures([
//                 self.interp_aux(i).map.dom().contains(self.base_vaddr + j * self.entry_size()),
//                 // equal(self.interp_aux(i).map.index(self.base_vaddr + i * self.entry_size()), self.entries(i))
//         ]);
// 
//         if i >= self.entries.len() {
//         } else {
//             if j == i {
//                 assume(false);
//             } else {
//                 assert(j > i);
//                 // assert(self.inv());
//                 reveal_with_fuel(Self::interp_aux, 5);
//                 assert(j >= i +1);
//                 assert(j < self.entries.len());
//                 assert(self.entries.index(j).is_Page());
//                 self.x1(j, i+1);
//                 assert(self.layer as int + 1 < self.arch.layer_sizes.len());
//                 assert(self.interp_aux(i+1).map.dom().contains(self.base_vaddr + j * self.entry_size()));
//                 // assume(self.interp_aux(i+1).map.dom().contains(self.base_vaddr + j * self.entry_size()));
//                 let rem = self.interp_aux(i+1).map;
//                 match self.entries.index(i) {
//                     NodeEntry::Page(p) => {
//                         assume(false);
//                         assume(self.entry_size() > 0);
//                         assert(self.base_vaddr + i * self.entry_size() < self.base_vaddr + j * self.entry_size());
//                         assert(rem.union_prefer_right(map![self.base_vaddr + i * self.entry_size() => p]).dom().contains(self.base_vaddr + j * self.entry_size()));
//                     },
//                     NodeEntry::Directory(d) => {
//                         assume(false);
//                         let dmap = d.interp_aux(0).map;
//                         assume(forall(|k:nat|
//                                       dmap.dom().contains(k)
//                                       >>= self.base_vaddr + i * self.entry_size() <= k
//                                       && k + dmap.index(k).size <= self.base_vaddr + (i+1) * self.entry_size()));
//                         assert(!dmap.dom().contains(self.base_vaddr + j * self.entry_size()));
//                     },
//                 };
//             }
//         }
//     }
// 
// 
// }
// 
// 
// #[proof]
// pub fn lemma_set_contains_IMP_len_greater_zero<T>(s: Set<T>, a: T) {
//     requires([
//              s.finite(),
//              s.contains(a)
//     ]);
//     ensures(s.len() > 0);
// 
//     if s.len() == 0 {
//         // contradiction
//         assert(s.remove(a).len() + 1 == 0);
//     }
// }
