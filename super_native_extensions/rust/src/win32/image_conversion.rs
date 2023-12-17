use std::{ptr::null_mut, slice};

use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{HGLOBAL, VARIANT_BOOL},
        Graphics::Imaging::{
            CLSID_WICImagingFactory, GUID_ContainerFormatBmp, GUID_ContainerFormatPng,
            IWICBitmapFrameEncode, IWICImagingFactory, WICBitmapEncoderNoCache,
        },
        System::{
            Com::{
                IStream,
                StructuredStorage::{
                    CreateStreamOnHGlobal, GetHGlobalFromStream, IPropertyBag2, PROPBAG2,
                },
            },
            Memory::{GlobalLock, GlobalSize, GlobalUnlock},
            Variant::{VariantInit, VT_BOOL},
        },
    },
};

use super::common::create_instance;

/// Convert image from input_stream to PNG
pub fn convert_to_png(input_stream: IStream) -> windows::core::Result<Vec<u8>> {
    let factory: IWICImagingFactory = create_instance(&CLSID_WICImagingFactory)?;
    unsafe {
        let decoder =
            factory.CreateDecoderFromStream(&input_stream, null_mut(), Default::default())?;
        let encoder = factory.CreateEncoder(&GUID_ContainerFormatPng, null_mut())?;
        let output_stream = CreateStreamOnHGlobal(HGLOBAL::default(), true)?;
        encoder.Initialize(&output_stream, WICBitmapEncoderNoCache)?;
        let frame = decoder.GetFrame(0)?;
        let mut encoder_frame = Option::<IWICBitmapFrameEncode>::None;
        encoder.CreateNewFrame(&mut encoder_frame as *mut _, null_mut())?;
        let encoder_frame = encoder_frame.unwrap();
        encoder_frame.Initialize(None)?;
        encoder_frame.WriteSource(&frame, std::ptr::null_mut())?;
        encoder_frame.Commit()?;
        encoder.Commit()?;
        let hglobal = GetHGlobalFromStream(&output_stream)?;
        let size = GlobalSize(hglobal);
        let data = GlobalLock(hglobal);
        let v = slice::from_raw_parts(data as *const u8, size);
        let res: Vec<u8> = v.into();
        GlobalUnlock(hglobal).ok();
        // prevent clippy from complaining. want the stream to outlive
        // hglobal
        let _output_stream = output_stream;
        Ok(res)
    }
}

/// Converts image from input stream to CF_DIB or CF_DIBV5 representation.
pub fn convert_to_dib(input_stream: IStream, use_v5: bool) -> windows::core::Result<Vec<u8>> {
    let factory: IWICImagingFactory = create_instance(&CLSID_WICImagingFactory)?;
    unsafe {
        let decoder =
            factory.CreateDecoderFromStream(&input_stream, null_mut(), Default::default())?;
        let encoder = factory.CreateEncoder(&GUID_ContainerFormatBmp, null_mut())?;
        let output_stream = CreateStreamOnHGlobal(HGLOBAL::default(), true)?;
        encoder.Initialize(&output_stream, WICBitmapEncoderNoCache)?;
        let frame = decoder.GetFrame(0)?;
        let mut encoder_frame = Option::<IWICBitmapFrameEncode>::None;
        let mut property_bag = Option::<IPropertyBag2>::None;
        encoder.CreateNewFrame(&mut encoder_frame as *mut _, &mut property_bag as *mut _)?;
        if let Some(property_bag) = property_bag.as_mut() {
            if use_v5 {
                let mut option = PROPBAG2::default();
                let mut name: Vec<_> = "EnableV5Header32bppBGRA".encode_utf16().collect();
                name.push(0);
                option.pstrName = PWSTR(name.as_ptr() as *mut _);
                let mut variant = VariantInit();
                let inside = &mut variant.Anonymous.Anonymous;
                inside.vt = VT_BOOL;
                inside.Anonymous.boolVal = VARIANT_BOOL(0xFFFFu16 as i16);
                property_bag.Write(1, &mut option as *mut _, &mut variant as *mut _)?;
            }
        }
        let encoder_frame = encoder_frame.unwrap();
        encoder_frame.Initialize(&property_bag.unwrap())?;
        encoder_frame.WriteSource(&frame, std::ptr::null_mut())?;
        encoder_frame.Commit()?;
        encoder.Commit()?;
        let hglobal = GetHGlobalFromStream(&output_stream)?;
        let size = GlobalSize(hglobal);
        let data = GlobalLock(hglobal);
        let v = slice::from_raw_parts(data as *const u8, size);
        // Strip BMP file header
        let sub = &v[14..];
        let res: Vec<u8> = sub.into();
        GlobalUnlock(hglobal).ok();
        // prevent clippy from complaining. want the stream to outlive
        // hglobal
        let _output_stream = output_stream;
        Ok(res)
    }
}
