use windows_sys::Win32::System::LibraryLoader::GetModuleHandleA;

pub enum RenderingAPI {
    Dx11,
    Dx12
}

// This is beyond a good ortodox way of detecting this but we can safely rely on this for now.
pub fn detect_api() -> RenderingAPI {
    let dx12 = unsafe { GetModuleHandleA("D3D12Core.dll\0".as_ptr() as _) };

    if dx12 == 0 {
        return RenderingAPI::Dx11;
    }

    return RenderingAPI::Dx12;
}
