use wdk_sys::{KPROCESSOR_MODE, NTSTATUS, PEPROCESS, PIO_STACK_LOCATION, PIRP, PVOID};

#[allow(non_snake_case)]
pub unsafe fn IoGetCurrentIrpStackLocation(p_irp: PIRP) -> PIO_STACK_LOCATION {
    assert!((*p_irp).CurrentLocation <= (*p_irp).StackCount + 1);
    (*p_irp)
        .Tail
        .Overlay
        .__bindgen_anon_2
        .__bindgen_anon_1
        .CurrentStackLocation
}

#[allow(non_snake_case)]
extern "C" {
    pub fn MmCopyVirtualMemory(
        SourceProcess: PEPROCESS,
        SourceAddress: PVOID,
        TargetProcess: PEPROCESS,
        TargetAddress: PVOID,
        BufferSize: usize,
        PreviousMode: KPROCESSOR_MODE,
        ReturnSize: &mut usize,
    ) -> NTSTATUS;
}
