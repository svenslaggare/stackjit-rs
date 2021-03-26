use crate::runtime::memory::heap::{Heap, HeapObjectsIterator};
use crate::runtime::stack_walker::{StackFrame, FrameValue};
use crate::compiler::jit::JitCompiler;
use crate::engine::binder::Binder;
use crate::model::typesystem::Type;
use crate::runtime::object::ObjectReference;
use crate::runtime::array;

pub struct GarbageCollector {
    deleted_objects: Vec<(u64, Type)>
}

impl GarbageCollector {
    pub fn new() -> GarbageCollector {
        GarbageCollector {
            deleted_objects: Vec::new()
        }
    }

    pub fn deleted_objects(&self) -> &Vec<(u64, Type)> {
        &self.deleted_objects
    }

    pub fn collect(&mut self,
                   compiler: &JitCompiler,
                   binder: &Binder,
                   heap: &mut Heap,
                   stack_frame: StackFrame) {
        let print_objects = || {
            for object_ref in HeapObjectsIterator::new(&heap) {
                println!(
                    "0x{:0x} - type: {}, size: {}, marked: {}, dead: {}",
                    object_ref.ptr() as u64,
                    object_ref.object_type().instance,
                    object_ref.size(),
                    object_ref.header().is_marked(),
                    object_ref.header().is_deleted()
                );
            }
        };

        println!("--------------------------------------------");

        println!("Stack values:");
        stack_frame.walk(
            compiler,
            binder,
            |frame| {
                frame.print_frame();
                println!();
            }
        );

        println!();
        println!("Before heap objects:");
        print_objects();
        println!();

        self.mark_objects(compiler, binder, &stack_frame);
        self.sweep_objects(heap);

        println!();
        println!("After heap objects:");
        print_objects();

        println!("--------------------------------------------");
    }

    fn mark_objects(&mut self,
                    compiler: &JitCompiler,
                    binder: &Binder,
                    stack_frame: &StackFrame) {
        stack_frame.walk(
            compiler,
            binder,
            |frame| {
                frame.visit_values(|value| {
                    self.mark_value(value);
                });
            }
        );
    }

    fn mark_value(&mut self, value: FrameValue) {
        if value.value_type.is_reference() {
            if value.value_u64() != 0 {
                let mut object_ref = ObjectReference::from_ptr(value.value_u64() as *const u8).unwrap();

                if !object_ref.header().is_marked() {
                    object_ref.header_mut().mark();

                    match value.value_type {
                        Type::Array(element) => {
                            if element.is_reference() {
                                let array_length = array::get_length(object_ref.ptr());
                                let elements_ptr = array::get_elements::<u64>(object_ref.ptr());

                                for index in 0..array_length {
                                    self.mark_value(
                                        FrameValue::new_value(
                                            element,
                                            unsafe { elements_ptr.add(index) as *const u8 }
                                        )
                                    );
                                }
                            }
                        }
                        Type::Class(_) => {
                            for field in object_ref.object_type().class.as_ref().unwrap().fields() {
                                if field.field_type().is_reference() {
                                    self.mark_value(
                                        FrameValue::new_value(
                                            field.field_type(),
                                            unsafe { object_ref.ptr().add(field.offset()) as *const u8 }
                                        )
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn sweep_objects(&mut self, heap: &Heap) {
        for mut object_ref in HeapObjectsIterator::new(heap) {
            if !object_ref.header().is_marked() {
                println!("Deleted object: 0x{:0x}, type: {}", object_ref.ptr() as u64, object_ref.object_type().instance);
                self.deleted_objects.push((object_ref.ptr() as u64, object_ref.object_type().instance.clone()));
                object_ref.delete();
            } else {
                object_ref.header_mut().unmark();
            }
        }
    }
}