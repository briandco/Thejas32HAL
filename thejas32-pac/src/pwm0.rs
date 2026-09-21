#[repr(C)]
#[derive(Debug)]
#[doc = "Register block"]
pub struct RegisterBlock {
    control: Control,
    status: Status,
    period: Period,
    on_off: OnOff,
}
impl RegisterBlock {
    #[doc = "0x00 - PWM channel control register"]
    #[inline(always)]
    pub const fn control(&self) -> &Control {
        &self.control
    }
    #[doc = "0x04 - PWM channel status register"]
    #[inline(always)]
    pub const fn status(&self) -> &Status {
        &self.status
    }
    #[doc = "0x08 - PWM period value"]
    #[inline(always)]
    pub const fn period(&self) -> &Period {
        &self.period
    }
    #[doc = "0x0c - PWM on/off time value"]
    #[inline(always)]
    pub const fn on_off(&self) -> &OnOff {
        &self.on_off
    }
}
#[doc = "CONTROL (rw) register accessor: PWM channel control register\n\nYou can [`read`](crate::Reg::read) this register and get [`control::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`control::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@control`] module"]
#[doc(alias = "CONTROL")]
pub type Control = crate::Reg<control::ControlSpec>;
#[doc = "PWM channel control register"]
pub mod control;
#[doc = "STATUS (r) register accessor: PWM channel status register\n\nYou can [`read`](crate::Reg::read) this register and get [`status::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@status`] module"]
#[doc(alias = "STATUS")]
pub type Status = crate::Reg<status::StatusSpec>;
#[doc = "PWM channel status register"]
pub mod status;
#[doc = "PERIOD (rw) register accessor: PWM period value\n\nYou can [`read`](crate::Reg::read) this register and get [`period::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`period::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@period`] module"]
#[doc(alias = "PERIOD")]
pub type Period = crate::Reg<period::PeriodSpec>;
#[doc = "PWM period value"]
pub mod period;
#[doc = "ON_OFF (rw) register accessor: PWM on/off time value\n\nYou can [`read`](crate::Reg::read) this register and get [`on_off::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`on_off::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@on_off`] module"]
#[doc(alias = "ON_OFF")]
pub type OnOff = crate::Reg<on_off::OnOffSpec>;
#[doc = "PWM on/off time value"]
pub mod on_off;
