#[doc = "Register `FCR` writer"]
pub type W = crate::W<FcrSpec>;
#[doc = "Field `FIFO_EN` writer - FIFO enable"]
pub type FifoEnW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `RX_FIFO_RST` writer - Receive FIFO reset"]
pub type RxFifoRstW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `TX_FIFO_RST` writer - Transmit FIFO reset"]
pub type TxFifoRstW<'a, REG> = crate::BitWriter<'a, REG>;
impl core::fmt::Debug for crate::generic::Reg<FcrSpec> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(not readable)")
    }
}
impl W {
    #[doc = "Bit 0 - FIFO enable"]
    #[inline(always)]
    pub fn fifo_en(&mut self) -> FifoEnW<'_, FcrSpec> {
        FifoEnW::new(self, 0)
    }
    #[doc = "Bit 1 - Receive FIFO reset"]
    #[inline(always)]
    pub fn rx_fifo_rst(&mut self) -> RxFifoRstW<'_, FcrSpec> {
        RxFifoRstW::new(self, 1)
    }
    #[doc = "Bit 2 - Transmit FIFO reset"]
    #[inline(always)]
    pub fn tx_fifo_rst(&mut self) -> TxFifoRstW<'_, FcrSpec> {
        TxFifoRstW::new(self, 2)
    }
}
#[doc = "FIFO Control Register\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`fcr::W`](W). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct FcrSpec;
impl crate::RegisterSpec for FcrSpec {
    type Ux = u32;
}
#[doc = "`write(|w| ..)` method takes [`fcr::W`](W) writer structure"]
impl crate::Writable for FcrSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets FCR to value 0"]
impl crate::Resettable for FcrSpec {}
