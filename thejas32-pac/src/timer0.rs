#[repr(C)]
#[derive(Debug)]
#[doc = "Register block"]
pub struct RegisterBlock {
    load_count: LoadCount,
    current_value: CurrentValue,
    control: Control,
    eoi: Eoi,
    intr_status: IntrStatus,
}
impl RegisterBlock {
    #[doc = "0x00 - Timer load count value"]
    #[inline(always)]
    pub const fn load_count(&self) -> &LoadCount {
        &self.load_count
    }
    #[doc = "0x04 - Timer current value"]
    #[inline(always)]
    pub const fn current_value(&self) -> &CurrentValue {
        &self.current_value
    }
    #[doc = "0x08 - Timer control register"]
    #[inline(always)]
    pub const fn control(&self) -> &Control {
        &self.control
    }
    #[doc = "0x0c - Timer end-of-interrupt (read clears interrupt)"]
    #[inline(always)]
    pub const fn eoi(&self) -> &Eoi {
        &self.eoi
    }
    #[doc = "0x10 - Timer interrupt status"]
    #[inline(always)]
    pub const fn intr_status(&self) -> &IntrStatus {
        &self.intr_status
    }
}
#[doc = "LOAD_COUNT (rw) register accessor: Timer load count value\n\nYou can [`read`](crate::Reg::read) this register and get [`load_count::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`load_count::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@load_count`] module"]
#[doc(alias = "LOAD_COUNT")]
pub type LoadCount = crate::Reg<load_count::LoadCountSpec>;
#[doc = "Timer load count value"]
pub mod load_count;
#[doc = "CURRENT_VALUE (r) register accessor: Timer current value\n\nYou can [`read`](crate::Reg::read) this register and get [`current_value::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@current_value`] module"]
#[doc(alias = "CURRENT_VALUE")]
pub type CurrentValue = crate::Reg<current_value::CurrentValueSpec>;
#[doc = "Timer current value"]
pub mod current_value;
#[doc = "CONTROL (rw) register accessor: Timer control register\n\nYou can [`read`](crate::Reg::read) this register and get [`control::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`control::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@control`] module"]
#[doc(alias = "CONTROL")]
pub type Control = crate::Reg<control::ControlSpec>;
#[doc = "Timer control register"]
pub mod control;
#[doc = "EOI (r) register accessor: Timer end-of-interrupt (read clears interrupt)\n\nYou can [`read`](crate::Reg::read) this register and get [`eoi::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@eoi`] module"]
#[doc(alias = "EOI")]
pub type Eoi = crate::Reg<eoi::EoiSpec>;
#[doc = "Timer end-of-interrupt (read clears interrupt)"]
pub mod eoi;
#[doc = "INTR_STATUS (r) register accessor: Timer interrupt status\n\nYou can [`read`](crate::Reg::read) this register and get [`intr_status::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@intr_status`] module"]
#[doc(alias = "INTR_STATUS")]
pub type IntrStatus = crate::Reg<intr_status::IntrStatusSpec>;
#[doc = "Timer interrupt status"]
pub mod intr_status;
