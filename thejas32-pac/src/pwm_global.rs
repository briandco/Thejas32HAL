#[repr(C)]
#[derive(Debug)]
#[doc = "Register block"]
pub struct RegisterBlock {
    gcr: Gcr,
}
impl RegisterBlock {
    #[doc = "0x00 - PWM Global Control Register"]
    #[inline(always)]
    pub const fn gcr(&self) -> &Gcr {
        &self.gcr
    }
}
#[doc = "GCR (rw) register accessor: PWM Global Control Register\n\nYou can [`read`](crate::Reg::read) this register and get [`gcr::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`gcr::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@gcr`] module"]
#[doc(alias = "GCR")]
pub type Gcr = crate::Reg<gcr::GcrSpec>;
#[doc = "PWM Global Control Register"]
pub mod gcr;
