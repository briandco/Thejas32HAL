#[doc = "Register `CONTROL` reader"]
pub type R = crate::R<ControlSpec>;
#[doc = "Register `CONTROL` writer"]
pub type W = crate::W<ControlSpec>;
#[doc = "Field `MODE` reader - Mode (0=idle, 1=one-shot, 2=continuous)"]
pub type ModeR = crate::FieldReader;
#[doc = "Field `MODE` writer - Mode (0=idle, 1=one-shot, 2=continuous)"]
pub type ModeW<'a, REG> = crate::FieldWriter<'a, REG, 2>;
#[doc = "Field `ALIGN` reader - Alignment control (0=left, 1=right)"]
pub type AlignR = crate::FieldReader;
#[doc = "Field `ALIGN` writer - Alignment control (0=left, 1=right)"]
pub type AlignW<'a, REG> = crate::FieldWriter<'a, REG, 2>;
#[doc = "Field `IE` reader - Interrupt enable"]
pub type IeR = crate::BitReader;
#[doc = "Field `IE` writer - Interrupt enable"]
pub type IeW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `OPC` reader - Output polarity control"]
pub type OpcR = crate::BitReader;
#[doc = "Field `OPC` writer - Output polarity control"]
pub type OpcW<'a, REG> = crate::BitWriter<'a, REG>;
#[doc = "Field `REPEAT_COUNT` reader - Repeat count for one-shot mode"]
pub type RepeatCountR = crate::FieldReader<u16>;
#[doc = "Field `REPEAT_COUNT` writer - Repeat count for one-shot mode"]
pub type RepeatCountW<'a, REG> = crate::FieldWriter<'a, REG, 16, u16>;
impl R {
    #[doc = "Bits 0:1 - Mode (0=idle, 1=one-shot, 2=continuous)"]
    #[inline(always)]
    pub fn mode(&self) -> ModeR {
        ModeR::new((self.bits & 3) as u8)
    }
    #[doc = "Bits 2:3 - Alignment control (0=left, 1=right)"]
    #[inline(always)]
    pub fn align(&self) -> AlignR {
        AlignR::new(((self.bits >> 2) & 3) as u8)
    }
    #[doc = "Bit 4 - Interrupt enable"]
    #[inline(always)]
    pub fn ie(&self) -> IeR {
        IeR::new(((self.bits >> 4) & 1) != 0)
    }
    #[doc = "Bit 5 - Output polarity control"]
    #[inline(always)]
    pub fn opc(&self) -> OpcR {
        OpcR::new(((self.bits >> 5) & 1) != 0)
    }
    #[doc = "Bits 6:21 - Repeat count for one-shot mode"]
    #[inline(always)]
    pub fn repeat_count(&self) -> RepeatCountR {
        RepeatCountR::new(((self.bits >> 6) & 0xffff) as u16)
    }
}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("CONTROL")
            .field("mode", &self.mode())
            .field("align", &self.align())
            .field("ie", &self.ie())
            .field("opc", &self.opc())
            .field("repeat_count", &self.repeat_count())
            .finish()
    }
}
impl W {
    #[doc = "Bits 0:1 - Mode (0=idle, 1=one-shot, 2=continuous)"]
    #[inline(always)]
    pub fn mode(&mut self) -> ModeW<'_, ControlSpec> {
        ModeW::new(self, 0)
    }
    #[doc = "Bits 2:3 - Alignment control (0=left, 1=right)"]
    #[inline(always)]
    pub fn align(&mut self) -> AlignW<'_, ControlSpec> {
        AlignW::new(self, 2)
    }
    #[doc = "Bit 4 - Interrupt enable"]
    #[inline(always)]
    pub fn ie(&mut self) -> IeW<'_, ControlSpec> {
        IeW::new(self, 4)
    }
    #[doc = "Bit 5 - Output polarity control"]
    #[inline(always)]
    pub fn opc(&mut self) -> OpcW<'_, ControlSpec> {
        OpcW::new(self, 5)
    }
    #[doc = "Bits 6:21 - Repeat count for one-shot mode"]
    #[inline(always)]
    pub fn repeat_count(&mut self) -> RepeatCountW<'_, ControlSpec> {
        RepeatCountW::new(self, 6)
    }
}
#[doc = "PWM channel control register\n\nYou can [`read`](crate::Reg::read) this register and get [`control::R`](R). You can [`reset`](crate::Reg::reset), [`write`](crate::Reg::write), [`write_with_zero`](crate::Reg::write_with_zero) this register using [`control::W`](W). You can also [`modify`](crate::Reg::modify) this register. See [API](https://docs.rs/svd2rust/#read--modify--write-api)."]
pub struct ControlSpec;
impl crate::RegisterSpec for ControlSpec {
    type Ux = u32;
}
#[doc = "`read()` method returns [`control::R`](R) reader structure"]
impl crate::Readable for ControlSpec {}
#[doc = "`write(|w| ..)` method takes [`control::W`](W) writer structure"]
impl crate::Writable for ControlSpec {
    type Safety = crate::Unsafe;
}
#[doc = "`reset()` method sets CONTROL to value 0"]
impl crate::Resettable for ControlSpec {}
