#[doc = "Register `TXFF` writer"]
pub type W = crate::W<TxffSpec>;
impl core::fmt::Debug for crate::generic::Reg<TxffSpec> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(not readable)")
    }
}
impl W {}
#[doc = "Transmit FIFO\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`txff::W`](W). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct TxffSpec;
impl crate::RegisterSpec for TxffSpec {
    type Ux = u8;
}
#[doc = "`write(|w| ..)` method takes [`txff::W`](W) writer structure"]
impl crate::Writable for TxffSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets TXFF to value 0"]
impl crate::Resettable for TxffSpec {}
