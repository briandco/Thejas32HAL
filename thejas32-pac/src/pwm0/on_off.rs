#[doc = "Register `ON_OFF` reader"]
pub type R = crate::R<OnOffSpec>;
#[doc = "Register `ON_OFF` writer"]
pub type W = crate::W<OnOffSpec>;
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.bits())
    }
}
impl W {}
#[doc = "PWM on/off time value\n\nYou can [`read`](crate::Reg::read) this register and get [`on_off::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`on_off::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct OnOffSpec;
impl crate::RegisterSpec for OnOffSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`on_off::R`](R) reader structure"]
impl crate::Readable for OnOffSpec {}
#[doc = "`write(|w| ..)` method takes [`on_off::W`](W) writer structure"]
impl crate::Writable for OnOffSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets ON_OFF to value 0"]
impl crate::Resettable for OnOffSpec {}
