use glfw_sys::*;

use crate::utils::CheckVkError;
use crate::{Buffer, CommandPool, Device, Queue};

use std::ffi::c_void;
use std::mem::{size_of, MaybeUninit};
use std::ptr;

impl Buffer {
    pub fn new(device: &Device, size: u64, usage: u32, properties: u32) -> Self {
        let create_info = VkBufferCreateInfo {
            sType: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
            size,
            usage,
            sharingMode: VK_SHARING_MODE_EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe {
            let mut buffer = MaybeUninit::<VkBuffer>::uninit();
            vkCreateBuffer(device.as_raw(), &create_info, ptr::null_mut(), buffer.as_mut_ptr())
                .check_err("create buffer");
            buffer.assume_init()
        };

        let mem_requirements = unsafe {
            let mut requirements = MaybeUninit::<VkMemoryRequirements>::uninit();
            vkGetBufferMemoryRequirements(device.as_raw(), buffer, requirements.as_mut_ptr());
            requirements.assume_init()
        };

        let memory_type =
            find_memory_type(device.phys_device, mem_requirements.memoryTypeBits, properties)
                .expect("failed to find appropriate memory type");

        let alloc_info = VkMemoryAllocateInfo {
            sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
            allocationSize: mem_requirements.size,
            memoryTypeIndex: memory_type,
            ..Default::default()
        };

        let memory = unsafe {
            let mut memory = MaybeUninit::<VkDeviceMemory>::uninit();

            vkAllocateMemory(device.as_raw(), &alloc_info, ptr::null_mut(), memory.as_mut_ptr())
                .check_err("allocate memory");

            memory.assume_init()
        };

        unsafe {
            vkBindBufferMemory(device.as_raw(), buffer, memory, 0);
        }

        Self {
            buffer,
            memory,
            device: device.as_raw(),
        }
    }

    pub fn with_data<T: Copy>(
        device: &Device,
        command_pool: &CommandPool,
        queue: &Queue,
        usage: u32,
        data: &[T],
    ) -> Self {
        let size_bytes: u64 = (data.len() * size_of::<T>()).try_into().unwrap();

        let mut staging_buffer = Self::new(
            device,
            size_bytes,
            VK_BUFFER_USAGE_TRANSFER_SRC_BIT,
            VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT,
        );

        staging_buffer.upload_to_buffer_memory(data);

        let mut buffer = Self::new(
            device,
            size_bytes,
            usage | VK_BUFFER_USAGE_TRANSFER_DST_BIT,
            VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT,
        );

        buffer.copy_from_buffer(command_pool, queue, &staging_buffer, size_bytes);

        buffer
    }

    pub fn upload_to_buffer_memory<T: Copy>(&mut self, data: &[T]) {
        let size_bytes: u64 = (data.len() * size_of::<T>()).try_into().unwrap();

        let memory_range = VkMappedMemoryRange {
            sType: VK_STRUCTURE_TYPE_MAPPED_MEMORY_RANGE,
            memory: self.memory,
            offset: 0,
            size: size_bytes,
            ..Default::default()
        };

        unsafe {
            let ptr: *mut T = ptr::null_mut();
            let mut void_ptr = ptr.cast::<c_void>();

            vkMapMemory(self.device, self.memory, 0, size_bytes, 0, &mut void_ptr)
                .check_err("map memory");

            let out_ptr = void_ptr.cast::<T>();

            let slice = std::slice::from_raw_parts_mut(out_ptr, data.len());
            slice.copy_from_slice(data);

            vkFlushMappedMemoryRanges(self.device, 1, &memory_range)
                .check_err("flush mapped memory");

            vkUnmapMemory(self.device, self.memory);
        }
    }

    pub fn copy_from_buffer(
        &mut self,
        command_pool: &CommandPool,
        queue: &Queue,
        src: &Buffer,
        size: u64,
    ) {
        let mut cmd_buffer = command_pool.create_command_buffer();

        cmd_buffer.record_with_flags(VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT, |handle| {
            handle.copy_buffer_full(&src, self, size);
        });

        queue.submit(&cmd_buffer);

        queue.wait_idle();
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            vkDestroyBuffer(self.device, self.buffer, ptr::null());
            vkFreeMemory(self.device, self.memory, ptr::null());
        }
    }
}

fn find_memory_type(
    phys_device: VkPhysicalDevice,
    req_type: u32,
    req_properties: u32,
) -> Option<u32> {
    let mem_properties = unsafe {
        let mut properties = MaybeUninit::<VkPhysicalDeviceMemoryProperties>::uninit();
        vkGetPhysicalDeviceMemoryProperties(phys_device, properties.as_mut_ptr());
        properties.assume_init()
    };

    for i in 0..mem_properties.memoryTypeCount {
        if req_type & (1 << i) == 0 {
            continue;
        }

        if mem_properties.memoryTypes[i as usize].propertyFlags & req_properties == 0 {
            continue;
        }

        return Some(i);
    }

    None
}
