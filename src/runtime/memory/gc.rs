use crate::runtime::memory::heap::{Heap, HeapObjectsIterator};
use crate::runtime::stack_walker::{StackFrame, FrameValue};
use crate::compiler::jit::JitCompiler;
use crate::model::binder::Binder;
use crate::model::typesystem::TypeId;
use crate::runtime::object::ObjectReference;
use crate::runtime::{array, object};
use crate::runtime::array::ArrayReference;
use crate::runtime::object::ObjectPointer;
use std::collections::HashMap;

pub struct GarbageCollector {
    deleted_objects: Vec<(u64, TypeId)>
}

impl GarbageCollector {
    pub fn new() -> GarbageCollector {
        GarbageCollector {
            deleted_objects: Vec::new()
        }
    }

    pub fn deleted_objects(&self) -> &Vec<(u64, TypeId)> {
        &self.deleted_objects
    }

    pub fn collect(&mut self,
                   compiler: &JitCompiler,
                   binder: &Binder,
                   heap: &mut Heap,
                   stack_frame: StackFrame) {
        let print_objects = |heap: &Heap| {
            for object_ref in HeapObjectsIterator::new(&heap) {
                println!(
                    "0x{:0x} - type: {}, size: {}, marked: {}, dead: {}",
                    object_ref.ptr() as u64,
                    object_ref.object_type().id,
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
        print_objects(heap);
        println!();

        self.mark_objects(compiler, binder, &stack_frame);
        // self.sweep_objects(heap);
        self.compact_objects(compiler, binder, heap, &stack_frame);

        println!();
        println!("After heap objects:");
        print_objects(heap);

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
            if value.value_ptr() != std::ptr::null_mut() {
                self.mark_object(ObjectReference::from_ptr(value.value_ptr()).unwrap());
            }
        }
    }

    fn mark_object(&mut self, mut object_ref: ObjectReference) {
        if !object_ref.header().is_marked() {
            object_ref.header_mut().mark();

            match &object_ref.object_type().id {
                TypeId::Array(element) => {
                    if element.is_reference() {
                        let array_ref = ArrayReference::<u64>::new(object_ref.ptr());
                        for index in 0..array_ref.length() {
                            self.mark_value(
                                FrameValue::new_value(
                                    element.as_ref(),
                                    array_ref.get_raw(index)
                                )
                            );
                        }
                    }
                }
                TypeId::Class(_) => {
                    for field in object_ref.object_type().class.as_ref().unwrap().fields() {
                        if field.type_id().is_reference() {
                            self.mark_value(
                                FrameValue::new_value(
                                    field.type_id(),
                                    unsafe { object_ref.ptr().add(field.offset()) as *mut u8 }
                                )
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn sweep_objects(&mut self, heap: &Heap) {
        for mut object_ref in HeapObjectsIterator::new(heap) {
            if !object_ref.header().is_marked() {
                println!("Deleted object: 0x{:0x}, type: {}", object_ref.ptr() as u64, object_ref.object_type().id);
                self.deleted_objects.push((object_ref.ptr() as u64, object_ref.object_type().id.clone()));
                object_ref.delete();
            } else {
                object_ref.header_mut().unmark();
            }
        }
    }

    fn compact_objects(&mut self,
                       compiler: &JitCompiler,
                       binder: &Binder,
                       heap: &mut Heap,
                       stack_frame: &StackFrame) {
        let (next_object_offset, new_locations) = self.compute_new_locations(heap);

        self.update_stack_references(compiler, binder, stack_frame, &new_locations);
        self.update_heap_references(heap, &new_locations);

        self.move_objects(heap, &new_locations);

        println!("Decreased heap by {} bytes", heap.offset() as isize - next_object_offset as isize);
        heap.set_offset(next_object_offset);
    }

    fn compute_new_locations(&self, heap: &Heap) -> (usize, HashMap<ObjectPointer, ObjectPointer>) {
        let mut object_offset = 0;
        let mut new_locations = HashMap::new();

        for object_ref in HeapObjectsIterator::new(heap) {
            if object_ref.header().is_marked() {
                new_locations.insert(object_ref.full_ptr(), unsafe { heap.data().as_ptr().add(object_offset) } as ObjectPointer);
                object_offset += object_ref.full_size();
            }
        }

        (object_offset, new_locations)
    }

    fn update_stack_references(&self,
                               compiler: &JitCompiler,
                               binder: &Binder,
                               stack_frame: &StackFrame,
                               new_locations: &HashMap<ObjectPointer, ObjectPointer>) {
        stack_frame.walk(
            compiler,
            binder,
            |frame| {
                frame.visit_values(|value| {
                    if value.value_type.is_reference() {
                        self.update_reference(new_locations, value.ptr_mut() as *mut ObjectPointer);
                    }
                })
            }
        );
    }

    fn update_heap_references(&self,
                              heap: &Heap,
                              new_locations: &HashMap<ObjectPointer, ObjectPointer>) {
        for object_ref in HeapObjectsIterator::new(heap) {
            if object_ref.header().is_marked() {
                match &object_ref.object_type().id {
                    TypeId::Array(element) => {
                        if element.is_reference() {
                            let array_ref = ArrayReference::<u64>::new(object_ref.ptr());
                            for index in 0..array_ref.length() {
                                self.update_reference(new_locations, array_ref.get_raw(index) as *mut ObjectPointer);
                            }
                        }
                    }
                    TypeId::Class(_) => {
                        for field in object_ref.object_type().class.as_ref().unwrap().fields() {
                            if field.type_id().is_reference() {
                                self.update_reference(
                                    new_locations,
                                    unsafe { object_ref.ptr().add(field.offset()) as *const u8 }as *mut ObjectPointer
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn update_reference(&self,
                        new_locations: &HashMap<ObjectPointer, ObjectPointer>,
                        object_ref_ptr: *mut ObjectPointer) {
        unsafe {
            let object_ref = *object_ref_ptr as ObjectPointer;
            if object_ref != std::ptr::null_mut() {
                let old_address = object_ref.sub(object::HEADER_SIZE);
                let new_address = new_locations[&old_address];
                *object_ref_ptr = new_address.add(object::HEADER_SIZE);
            }
        }
    }

    fn move_objects(&mut self,
                    heap: &mut Heap,
                    new_locations: &HashMap<ObjectPointer, ObjectPointer>,) {
        for mut object_ref in HeapObjectsIterator::new(heap) {
            if object_ref.header().is_marked() {
                object_ref.header_mut().unmark();
                let new_address = new_locations[&object_ref.full_ptr()];

                unsafe {
                    object_ref.full_ptr().copy_to(new_address, object_ref.full_size());
                }
            } else {
                println!("Deleted object: 0x{:0x}, type: {}", object_ref.ptr() as u64, object_ref.object_type().id);
                self.deleted_objects.push((object_ref.ptr() as u64, object_ref.object_type().id.clone()));
            }
        }
    }
}