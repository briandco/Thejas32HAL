#[doc = "Register `ADDRESS` reader"]
pub type R = crate::R<AddressSpec>;
#[doc = "Register `ADDRESS` writer"]
pub type W = crate::W<AddressSpec>;
#[doc = "Field `CHANNEL` reader - ADC channel number (0-3)"]
pub type ChannelR = crate::FieldReader;
#[doc = "Field `CHANNEL` writer - ADC channel number (0-3)"]
pub type ChannelW<'a, REG> = crate::FieldWriter<'a, REG, 2>;
impl R {
    #[doc = "Bits 0:1 - ADC channel number (0-3)"]
    #[inline(always)]
    pub fn channel(&self) -> ChannelR {
        ChannelR::new((self.bits & 3) as u8)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("ADDRESS")
            .field("channel", &self.channel())
            .finish()
    }
}
impl W {
    #[doc = "Bits 0:1 - ADC channel number (0-3)"]
    #[inline(always)]
    pub fn channel(&mut self) -> ChannelW<'_, AddressSpec> {
        ChannelW::new(self, 0)
    }
}
#[doc = "ADC channel select register\n\nYou can [`read`](crate::Reg::read) this register and get [`address::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`address::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct AddressSpec;
impl crate::RegisterSpec for AddressSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`address::R`](R) reader structure"]
impl crate::Readable for AddressSpec {}
#[doc = "`write(|w| ..)` method takes [`address::W`](W) writer structure"]
impl crate::Writable for AddressSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets ADDRESS to value 0"]
impl crate::Resettable for AddressSpec {}
