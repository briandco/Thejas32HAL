#[doc = "Register `BAUDRATE` reader"]
pub type R = crate::R<BaudrateSpec>;
#[doc = "Register `BAUDRATE` writer"]
pub type W = crate::W<BaudrateSpec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
impl W {}
#[doc = "SPI Baud Rate Divisor\n\nYou can [`read`](crate::Reg::read) this register and get [`baudrate::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`baudrate::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct BaudrateSpec;
impl crate::RegisterSpec for BaudrateSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`baudrate::R`](R) reader structure"]
impl crate::Readable for BaudrateSpec {}
#[doc = "`write(|w| ..)` method takes [`baudrate::W`](W) writer structure"]
impl crate::Writable for BaudrateSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets BAUDRATE to value 0x60"]
impl crate::Resettable for BaudrateSpec {
    const RESET_VALUE: u32 = 0x60;
}
