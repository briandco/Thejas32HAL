#[repr(C)]
#[derive(Debug)]
#[doc = "Register block"]
pub struct RegisterBlock {
    _reserved0: [u8; 0xa8],
    raw_intr_status: RawIntrStatus,
}
impl RegisterBlock {
    #[doc = "0xa8 - Global raw timer interrupt status register"]
    #[inline(always)]
    pub const fn raw_intr_status(&self) -> &RawIntrStatus {
        &self.raw_intr_status
    }
}
#[doc = "RAW_INTR_STATUS (r) register accessor: Global raw timer interrupt status register\n\nYou can [`read`](crate::Reg::read) this register and get [`raw_intr_status::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@raw_intr_status`] module"]
#[doc(alias = "RAW_INTR_STATUS")]
pub type RawIntrStatus = crate::Reg<raw_intr_status::RawIntrStatusSpec>;
#[doc = "Global raw timer interrupt status register"]
pub mod raw_intr_status;
