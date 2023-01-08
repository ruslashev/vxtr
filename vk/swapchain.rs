use glfw_sys::*;

use crate::utils::CheckVkError;
use crate::{Device, Framebuffer, ImageView, Instance, RenderPass, Semaphore, Swapchain};

use std::mem::MaybeUninit;
use std::ptr;

impl Swapchain {
    pub fn from_device(device: &Device, instance: &Instance, verbose: bool) -> Self {
        let surface_format = choose_swapchain_surface_format(&device.swapchain_support.formats);
        let present_mode =
            choose_swapchain_present_mode(&device.swapchain_support.present_modes, verbose);
        let extent =
            choose_swapchain_extent(instance.glfw_window, device.swapchain_support.capabilities);

        let max_image_count = device.swapchain_support.capabilities.maxImageCount;
        let mut image_count = device.swapchain_support.capabilities.minImageCount + 1;

        if image_count > max_image_count && max_image_count != 0 {
            image_count = max_image_count;
        }

        let gfx_idx = device.queue_families.graphics.unwrap();
        let present_idx = device.queue_families.present.unwrap();
        let indices = [gfx_idx, present_idx];

        let (sharing_mode, qf_idx_count, qf_indices) = if gfx_idx == present_idx {
            (VK_SHARING_MODE_EXCLUSIVE, 0, ptr::null())
        } else {
            (VK_SHARING_MODE_CONCURRENT, 2, indices.as_ptr())
        };

        let create_info = VkSwapchainCreateInfoKHR {
            sType: VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR,
            surface: instance.surface,
            minImageCount: image_count,
            imageFormat: surface_format.format,
            imageColorSpace: surface_format.colorSpace,
            imageExtent: extent,
            imageArrayLayers: 1,
            imageUsage: VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
            imageSharingMode: sharing_mode,
            queueFamilyIndexCount: qf_idx_count,
            pQueueFamilyIndices: qf_indices,
            preTransform: device.swapchain_support.capabilities.currentTransform,
            compositeAlpha: VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR,
            presentMode: present_mode,
            clipped: 1,
            oldSwapchain: ptr::null_mut(),
            ..Default::default()
        };

        let raw = unsafe {
            let mut swapchain = MaybeUninit::<VkSwapchainKHR>::uninit();

            vkCreateSwapchainKHR(
                device.as_raw(),
                &create_info,
                ptr::null(),
                swapchain.as_mut_ptr(),
            )
            .check_err("create swapchain");

            swapchain.assume_init()
        };

        Self {
            raw,
            format: surface_format.format,
            extent,
            device: device.as_raw(),
        }
    }

    pub fn get_image_views(&self) -> Vec<ImageView> {
        let images = unsafe {
            let mut count = 0;
            vkGetSwapchainImagesKHR(self.device, self.raw, &mut count, ptr::null_mut());

            let mut images = Vec::with_capacity(count as usize);
            images.resize(count as usize, ptr::null_mut());

            vkGetSwapchainImagesKHR(self.device, self.raw, &mut count, images.as_mut_ptr());

            images
        };

        let mut image_views = Vec::with_capacity(images.len());

        for image in images {
            let image_view = ImageView::from_raw(self.device, image, self.format);

            image_views.push(image_view);
        }

        image_views
    }

    pub fn acquire_next_image(&self, semaphore: &mut Semaphore, image_index: &mut u32) -> i32 {
        unsafe {
            vkAcquireNextImageKHR(
                self.device,
                self.raw,
                u64::MAX,
                semaphore.raw,
                ptr::null_mut(),
                image_index,
            )
        }
    }

    pub fn extent(&self) -> VkExtent2D {
        self.extent
    }

    pub fn format(&self) -> VkFormat {
        self.format
    }

    pub fn as_raw(&self) -> VkSwapchainKHR {
        self.raw
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            vkDestroySwapchainKHR(self.device, self.raw, ptr::null());
        }
    }
}

impl Framebuffer {
    pub fn new(
        device: &Device,
        render_pass: &RenderPass,
        image_view: &ImageView,
        swapchain: &Swapchain,
    ) -> Self {
        let create_info = VkFramebufferCreateInfo {
            sType: VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
            renderPass: render_pass.as_raw(),
            attachmentCount: 1,
            pAttachments: &image_view.as_raw(),
            width: swapchain.extent.width,
            height: swapchain.extent.height,
            layers: 1,
            ..Default::default()
        };

        let raw = unsafe {
            let mut fb = MaybeUninit::<VkFramebuffer>::uninit();

            vkCreateFramebuffer(device.as_raw(), &create_info, ptr::null_mut(), fb.as_mut_ptr())
                .check_err("create framebuffer");

            fb.assume_init()
        };

        Self {
            raw,
            device: device.as_raw(),
        }
    }

    pub fn as_raw(&self) -> VkFramebuffer {
        self.raw
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            vkDestroyFramebuffer(self.device, self.raw, ptr::null());
        }
    }
}

impl ImageView {
    fn from_raw(device: VkDevice, image: VkImage, image_format: VkFormat) -> Self {
        let create_info = VkImageViewCreateInfo {
            sType: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
            image,
            viewType: VK_IMAGE_VIEW_TYPE_2D,
            format: image_format,
            components: VkComponentMapping {
                r: VK_COMPONENT_SWIZZLE_IDENTITY,
                g: VK_COMPONENT_SWIZZLE_IDENTITY,
                b: VK_COMPONENT_SWIZZLE_IDENTITY,
                a: VK_COMPONENT_SWIZZLE_IDENTITY,
            },
            subresourceRange: VkImageSubresourceRange {
                aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                baseMipLevel: 0,
                levelCount: 1,
                baseArrayLayer: 0,
                layerCount: 1,
            },
            ..Default::default()
        };

        let raw = unsafe {
            let mut view = MaybeUninit::<VkImageView>::uninit();

            vkCreateImageView(device, &create_info, ptr::null(), view.as_mut_ptr())
                .check_err("create image view");

            view.assume_init()
        };

        Self { raw, device }
    }

    pub fn as_raw(&self) -> VkImageView {
        self.raw
    }
}

impl Drop for ImageView {
    fn drop(&mut self) {
        unsafe {
            vkDestroyImageView(self.device, self.raw, ptr::null());
        }
    }
}

fn choose_swapchain_surface_format(formats: &[VkSurfaceFormatKHR]) -> VkSurfaceFormatKHR {
    for format in formats {
        if format.format == VK_FORMAT_B8G8R8_SRGB
            && format.colorSpace == VK_COLOR_SPACE_SRGB_NONLINEAR_KHR
        {
            return *format;
        }
    }

    formats[0]
}

fn choose_swapchain_present_mode(
    present_modes: &[VkPresentModeKHR],
    verbose: bool,
) -> VkPresentModeKHR {
    if verbose {
        print_present_modes(present_modes);
    }

    let mode_priorities = [
        VK_PRESENT_MODE_IMMEDIATE_KHR,
        VK_PRESENT_MODE_FIFO_RELAXED_KHR,
        VK_PRESENT_MODE_MAILBOX_KHR,
        VK_PRESENT_MODE_FIFO_KHR,
    ];

    for mode in mode_priorities {
        if present_modes.iter().any(|m| *m == mode) {
            return mode;
        }
    }

    VK_PRESENT_MODE_FIFO_KHR
}

fn print_present_modes(present_modes: &[VkPresentModeKHR]) {
    println!("Present modes:");

    for mode in present_modes {
        let desc = match *mode {
            VK_PRESENT_MODE_IMMEDIATE_KHR => "Immediate",
            VK_PRESENT_MODE_MAILBOX_KHR => "Mailbox",
            VK_PRESENT_MODE_FIFO_KHR => "FIFO",
            VK_PRESENT_MODE_FIFO_RELAXED_KHR => "FIFO relaxed",
            VK_PRESENT_MODE_SHARED_DEMAND_REFRESH_KHR => "Shared on-demand refresh",
            VK_PRESENT_MODE_SHARED_CONTINUOUS_REFRESH_KHR => "Shared continuous refresh",
            _ => "Unknown",
        };

        println!("\t{}", desc);
    }
}

fn choose_swapchain_extent(
    glfw_window: *mut GLFWwindow,
    capabilities: VkSurfaceCapabilitiesKHR,
) -> VkExtent2D {
    if capabilities.currentExtent.width != u32::MAX {
        return capabilities.currentExtent;
    }

    let mut fb_width = 0;
    let mut fb_height = 0;

    unsafe {
        glfwGetFramebufferSize(glfw_window, &mut fb_width, &mut fb_height);
    }

    let fb_width: u32 = fb_width.try_into().unwrap();
    let fb_height: u32 = fb_height.try_into().unwrap();

    let min = capabilities.minImageExtent;
    let max = capabilities.maxImageExtent;

    VkExtent2D {
        width: fb_width.clamp(min.width, max.width),
        height: fb_height.clamp(min.height, max.height),
    }
}
