#[repr(C)]
#[derive(Debug)]
#[doc = "Register block"]
pub struct RegisterBlock {
    raw_intr: RawIntr,
    reserved0: Reserved0,
    intr_en: IntrEn,
    reserved1: Reserved1,
    intr_status: IntrStatus,
    _reserved5: [u8; 0x03ec],
    sw_intr_en: SwIntrEn,
}
impl RegisterBlock {
    #[doc = "0x00 - Raw interrupt status"]
    #[inline(always)]
    pub const fn raw_intr(&self) -> &RawIntr {
        &self.raw_intr
    }
    #[doc = "0x04 - Reserved"]
    #[inline(always)]
    pub const fn reserved0(&self) -> &Reserved0 {
        &self.reserved0
    }
    #[doc = "0x08 - Interrupt enable register"]
    #[inline(always)]
    pub const fn intr_en(&self) -> &IntrEn {
        &self.intr_en
    }
    #[doc = "0x0c - Reserved"]
    #[inline(always)]
    pub const fn reserved1(&self) -> &Reserved1 {
        &self.reserved1
    }
    #[doc = "0x10 - Masked interrupt status"]
    #[inline(always)]
    pub const fn intr_status(&self) -> &IntrStatus {
        &self.intr_status
    }
    #[doc = "0x400 - Software interrupt enable"]
    #[inline(always)]
    pub const fn sw_intr_en(&self) -> &SwIntrEn {
        &self.sw_intr_en
    }
}
#[doc = "RAW_INTR (r) register accessor: Raw interrupt status\n\nYou can [`read`](crate::Reg::read) this register and get [`raw_intr::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@raw_intr`] module"]
#[doc(alias = "RAW_INTR")]
pub type RawIntr = crate::Reg<raw_intr::RawIntrSpec>;
#[doc = "Raw interrupt status"]
pub mod raw_intr;
#[doc = "RESERVED0 (r) register accessor: Reserved\n\nYou can [`read`](crate::Reg::read) this register and get [`reserved0::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@reserved0`] module"]
#[doc(alias = "RESERVED0")]
pub type Reserved0 = crate::Reg<reserved0::Reserved0Spec>;
#[doc = "Reserved"]
pub mod reserved0;
#[doc = "INTR_EN (rw) register accessor: Interrupt enable register\n\nYou can [`read`](crate::Reg::read) this register and get [`intr_en::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`intr_en::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@intr_en`] module"]
#[doc(alias = "INTR_EN")]
pub type IntrEn = crate::Reg<intr_en::IntrEnSpec>;
#[doc = "Interrupt enable register"]
pub mod intr_en;
#[doc = "RESERVED1 (r) register accessor: Reserved\n\nYou can [`read`](crate::Reg::read) this register and get [`reserved1::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@reserved1`] module"]
#[doc(alias = "RESERVED1")]
pub type Reserved1 = crate::Reg<reserved1::Reserved1Spec>;
#[doc = "Reserved"]
pub mod reserved1;
#[doc = "INTR_STATUS (r) register accessor: Masked interrupt status\n\nYou can [`read`](crate::Reg::read) this register and get [`intr_status::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@intr_status`] module"]
#[doc(alias = "INTR_STATUS")]
pub type IntrStatus = crate::Reg<intr_status::IntrStatusSpec>;
#[doc = "Masked interrupt status"]
pub mod intr_status;
#[doc = "SW_INTR_EN (rw) register accessor: Software interrupt enable\n\nYou can [`read`](crate::Reg::read) this register and get [`sw_intr_en::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`sw_intr_en::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@sw_intr_en`] module"]
#[doc(alias = "SW_INTR_EN")]
pub type SwIntrEn = crate::Reg<sw_intr_en::SwIntrEnSpec>;
#[doc = "Software interrupt enable"]
pub mod sw_intr_en;
