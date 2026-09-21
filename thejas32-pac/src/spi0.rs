#[repr(C)]
#[derive(Debug)]
#[doc = "Register block"]
pub struct RegisterBlock {
    control: Control,
    status: Status,
    baudrate: Baudrate,
    txdata: Txdata,
    rxdata: Rxdata,
}
impl RegisterBlock {
    #[doc = "0x00 - SPI Control Register"]
    #[inline(always)]
    pub const fn control(&self) -> &Control {
        &self.control
    }
    #[doc = "0x04 - SPI Status Register"]
    #[inline(always)]
    pub const fn status(&self) -> &Status {
        &self.status
    }
    #[doc = "0x08 - SPI Baud Rate Divisor"]
    #[inline(always)]
    pub const fn baudrate(&self) -> &Baudrate {
        &self.baudrate
    }
    #[doc = "0x0c - SPI Transmit Data Register"]
    #[inline(always)]
    pub const fn txdata(&self) -> &Txdata {
        &self.txdata
    }
    #[doc = "0x10 - SPI Receive Data Register"]
    #[inline(always)]
    pub const fn rxdata(&self) -> &Rxdata {
        &self.rxdata
    }
}
#[doc = "CONTROL (rw) register accessor: SPI Control Register\n\nYou can [`read`](crate::Reg::read) this register and get [`control::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`control::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@control`] module"]
#[doc(alias = "CONTROL")]
pub type Control = crate::Reg<control::ControlSpec>;
#[doc = "SPI Control Register"]
pub mod control;
#[doc = "STATUS (r) register accessor: SPI Status Register\n\nYou can [`read`](crate::Reg::read) this register and get [`status::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@status`] module"]
#[doc(alias = "STATUS")]
pub type Status = crate::Reg<status::StatusSpec>;
#[doc = "SPI Status Register"]
pub mod status;
#[doc = "BAUDRATE (rw) register accessor: SPI Baud Rate Divisor\n\nYou can [`read`](crate::Reg::read) this register and get [`baudrate::R`]. You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`baudrate::W`]. You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@baudrate`] module"]
#[doc(alias = "BAUDRATE")]
pub type Baudrate = crate::Reg<baudrate::BaudrateSpec>;
#[doc = "SPI Baud Rate Divisor"]
pub mod baudrate;
#[doc = "TXDATA (w) register accessor: SPI Transmit Data Register\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`txdata::W`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@txdata`] module"]
#[doc(alias = "TXDATA")]
pub type Txdata = crate::Reg<txdata::TxdataSpec>;
#[doc = "SPI Transmit Data Register"]
pub mod txdata;
#[doc = "RXDATA (r) register accessor: SPI Receive Data Register\n\nYou can [`read`](crate::Reg::read) this register and get [`rxdata::R`]. See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [`mod@rxdata`] module"]
#[doc(alias = "RXDATA")]
pub type Rxdata = crate::Reg<rxdata::RxdataSpec>;
#[doc = "SPI Receive Data Register"]
pub mod rxdata;
