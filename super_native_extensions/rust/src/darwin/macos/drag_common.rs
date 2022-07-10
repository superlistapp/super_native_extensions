use cocoa::foundation::NSUInteger;

pub type NSDragOperation = NSUInteger;

#[allow(non_upper_case_globals)]
pub const NSDragOperationNone: NSDragOperation = 0;
#[allow(non_upper_case_globals)]
pub const NSDragOperationCopy: NSDragOperation = 1;
#[allow(non_upper_case_globals)]
pub const NSDragOperationLink: NSDragOperation = 2;
#[allow(non_upper_case_globals)]
pub const NSDragOperationMove: NSDragOperation = 16;
