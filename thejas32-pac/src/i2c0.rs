#[repr(C)]
#[derive(Debug)]
#[doc = "Register block"]
pub struct RegisterBlock {
    cr: Cr,
    sr0: Sr0,
    sr1: Sr1,
    ier: Ier,
    txff: Txff,
    rxff: Rxff,
    chl: Chl,
    chh: Chh,
    chhl: Chhl,
    chhh: Chhh,
    txclr: Txclr,
}
impl RegisterBlock {
    #[doc = "0x00 - Control Register"]
    #[inline(always)]
    pub const fn cr(&self) -> &Cr {
        &self.cr
    }
    #[doc = "0x01 - Status Register 0"]
    #[inline(always)]
    pub const fn sr0(&self) -> &Sr0 {
        &self.sr0
    }
    #[doc = "0x02 - Status Register 1"]
    #[inline(always)]
    pub const fn sr1(&self) -> &Sr1 {
        &self.sr1
    }
    #[doc = "0x03 - Interrupt Enable Register"]
    #[inline(always)]
    pub const fn ier(&self) -> &Ier {
        &self.ier
    }
    #[doc = "0x04 - Transmit FIFO"]
    #[inline(always)]
    pub const fn txff(&self) -> &Txff {
        &self.txff
    }
    #[doc = "0x05 - Receive FIFO"]
    #[inline(always)]
    pub const fn rxff(&self) -> &Rxff {
        &self.rxff
    }
    #[doc = "0x06 - Clock Low Period"]
    #[inline(always)]
    pub const fn chl(&self) -> &Chl {
        &self.chl
    }
    #[doc = "0x07 - Clock High Period"]
    #[inline(always)]
    pub const fn chh(&self) -> &Chh {
        &self.chh
    }
    #[doc = "0x08 - Clock Low Period (extended)"]
    #[inline(always)]
    pub const fn chhl(&self) -> &Chhl {
        &self.chhl
    }
    #[doc = "0x09 - Clock High Period (extended)"]
    #[inline(always)]
    pub const fn chhh(&self) -> &Chhh {
        &self.chhh
    }
    #[doc = "0x0a - TX Clear"]
    #[inline(always)]
    pub const fn txclr(&self) -> &Txclr {
        &self.txclr
    }
}
#[doc = "CR (rw) register accessor: Control Register\n\nYou can [`read`](crate::Reg::read) this register and get [`cr::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`cr::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@cr`] module"]
#[doc(alias = "CR")]
pub type Cr = crate::Reg<cr::CrSpec>;
#[doc = "Control Register"]
pub mod cr;
#[doc = "SR0 (r) register accessor: Status Register 0\n\nYou can [`read`](crate::Reg::read) this register and get [`sr0::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@sr0`] module"]
#[doc(alias = "SR0")]
pub type Sr0 = crate::Reg<sr0::Sr0Spec>;
#[doc = "Status Register 0"]
pub mod sr0;
#[doc = "SR1 (r) register accessor: Status Register 1\n\nYou can [`read`](crate::Reg::read) this register and get [`sr1::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@sr1`] module"]
#[doc(alias = "SR1")]
pub type Sr1 = crate::Reg<sr1::Sr1Spec>;
#[doc = "Status Register 1"]
pub mod sr1;
#[doc = "IER (rw) register accessor: Interrupt Enable Register\n\nYou can [`read`](crate::Reg::read) this register and get [`ier::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`ier::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@ier`] module"]
#[doc(alias = "IER")]
pub type Ier = crate::Reg<ier::IerSpec>;
#[doc = "Interrupt Enable Register"]
pub mod ier;
#[doc = "TXFF (w) register accessor: Transmit FIFO\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`txff::W`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@txff`] module"]
#[doc(alias = "TXFF")]
pub type Txff = crate::Reg<txff::TxffSpec>;
#[doc = "Transmit FIFO"]
pub mod txff;
#[doc = "RXFF (r) register accessor: Receive FIFO\n\nYou can [`read`](crate::Reg::read) this register and get [`rxff::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@rxff`] module"]
#[doc(alias = "RXFF")]
pub type Rxff = crate::Reg<rxff::RxffSpec>;
#[doc = "Receive FIFO"]
pub mod rxff;
#[doc = "CHL (rw) register accessor: Clock Low Period\n\nYou can [`read`](crate::Reg::read) this register and get [`chl::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`chl::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@chl`] module"]
#[doc(alias = "CHL")]
pub type Chl = crate::Reg<chl::ChlSpec>;
#[doc = "Clock Low Period"]
pub mod chl;
#[doc = "CHH (rw) register accessor: Clock High Period\n\nYou can [`read`](crate::Reg::read) this register and get [`chh::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`chh::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@chh`] module"]
#[doc(alias = "CHH")]
pub type Chh = crate::Reg<chh::ChhSpec>;
#[doc = "Clock High Period"]
pub mod chh;
#[doc = "CHHL (rw) register accessor: Clock Low Period (extended)\n\nYou can [`read`](crate::Reg::read) this register and get [`chhl::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`chhl::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@chhl`] module"]
#[doc(alias = "CHHL")]
pub type Chhl = crate::Reg<chhl::ChhlSpec>;
#[doc = "Clock Low Period (extended)"]
pub mod chhl;
#[doc = "CHHH (rw) register accessor: Clock High Period (extended)\n\nYou can [`read`](crate::Reg::read) this register and get [`chhh::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`chhh::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@chhh`] module"]
#[doc(alias = "CHHH")]
pub type Chhh = crate::Reg<chhh::ChhhSpec>;
#[doc = "Clock High Period (extended)"]
pub mod chhh;
#[doc = "TXCLR (w) register accessor: TX Clear\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`txclr::W`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@txclr`] module"]
#[doc(alias = "TXCLR")]
pub type Txclr = crate::Reg<txclr::TxclrSpec>;
#[doc = "TX Clear"]
pub mod txclr;
