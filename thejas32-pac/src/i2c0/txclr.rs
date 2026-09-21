#[doc = "Register `TXCLR` writer"]
pub type W = crate::W<TxclrSpec>;
impl core::fmt::Debug for crate::generic::Reg<TxclrSpec> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(not readable)")
    }
}
impl W {}
#[doc = "TX Clear\n\nYou can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`txclr::W`](W). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct TxclrSpec;
impl crate::RegisterSpec for TxclrSpec {
    type Ux = u8;
}
#[doc = "`write(|w| ..)` method takes [`txclr::W`](W) writer structure"]
impl crate::Writable for TxclrSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets TXCLR to value 0"]
impl crate::Resettable for TxclrSpec {}
