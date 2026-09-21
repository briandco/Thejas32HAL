#[doc = "Register `RXFF` reader"]
pub type R = crate::R<RxffSpec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
#[doc = "Receive FIFO\n\nYou can [`read`](crate::Reg::read) this register and get [`rxff::R`](R). See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct RxffSpec;
impl crate::RegisterSpec for RxffSpec {
    type Ux = u8;
}
#[doc = "`read()` method returns [`rxff::R`](R) reader structure"]
impl crate::Readable for RxffSpec {}
#[doc = "`reset()` method sets RXFF to value 0"]
impl crate::Resettable for RxffSpec {}
