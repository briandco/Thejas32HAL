#[doc = "Register `STATUS` reader"]
pub type R = crate::R<StatusSpec>;
#[doc = "Field `RX_INT_STATUS` reader - RX interrupt status"]
pub type RxIntStatusR = crate::BitReader;
#[doc = "Field `TX_INT_STATUS` reader - TX interrupt status"]
pub type TxIntStatusR = crate::BitReader;
#[doc = "Field `BUSY` reader - SPI busy"]
pub type BusyR = crate::BitReader;
#[doc = "Field `OVERRUN` reader - Overrun error"]
pub type OverrunR = crate::BitReader;
#[doc = "Field `RX_COMPLETE` reader - Receive complete"]
pub type RxCompleteR = crate::BitReader;
#[doc = "Field `TX_HOLD_EMPTY` reader - Transmit holding register empty"]
pub type TxHoldEmptyR = crate::BitReader;
impl R {
    #[doc = "Bit 2 - RX interrupt status"]
    #[inline(always)]
    pub fn rx_int_status(&self) -> RxIntStatusR {
        RxIntStatusR::new(((self.bits >> 2) & 1) != 0)
    }
    #[doc = "Bit 3 - TX interrupt status"]
    #[inline(always)]
    pub fn tx_int_status(&self) -> TxIntStatusR {
        TxIntStatusR::new(((self.bits >> 3) & 1) != 0)
    }
    #[doc = "Bit 4 - SPI busy"]
    #[inline(always)]
    pub fn busy(&self) -> BusyR {
        BusyR::new(((self.bits >> 4) & 1) != 0)
    }
    #[doc = "Bit 5 - Overrun error"]
    #[inline(always)]
    pub fn overrun(&self) -> OverrunR {
        OverrunR::new(((self.bits >> 5) & 1) != 0)
    }
    #[doc = "Bit 6 - Receive complete"]
    #[inline(always)]
    pub fn rx_complete(&self) -> RxCompleteR {
        RxCompleteR::new(((self.bits >> 6) & 1) != 0)
    }
    #[doc = "Bit 7 - Transmit holding register empty"]
    #[inline(always)]
    pub fn tx_hold_empty(&self) -> TxHoldEmptyR {
        TxHoldEmptyR::new(((self.bits >> 7) & 1) != 0)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("STATUS")
            .field("rx_int_status", &self.rx_int_status())
            .field("tx_int_status", &self.tx_int_status())
            .field("busy", &self.busy())
            .field("overrun", &self.overrun())
            .field("rx_complete", &self.rx_complete())
            .field("tx_hold_empty", &self.tx_hold_empty())
            .finish()
    }
}
#[doc = "SPI Status Register\n\nYou can [`read`](crate::Reg::read) this register and get [`status::R`](R). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct StatusSpec;
impl crate::RegisterSpec for StatusSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`status::R`](R) reader structure"]
impl crate::Readable for StatusSpec {}
#[doc = "`reset()` method sets STATUS to value 0x80"]
impl crate::Resettable for StatusSpec {
    const RESET_VALUE: u32 = 0x80;
}
