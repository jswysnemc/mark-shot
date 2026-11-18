//! 【共享内存池测试】【无密封与槽位契约】离线验证无密封 memfd、稀疏打洞重写及空闲块合并逻辑

use super::{
    FreelistEntry, coalesce_into_freelist,
    raw::{create_unsealed_shm_fd, protocol_pool_size},
    take_from_freelist,
};
use memmap2::MmapMut;
use std::{fs::File, io::ErrorKind, os::unix::io::AsRawFd};

/// 【共享内存池测试】【无密封与打洞】无参数，验证共享文件未启用密封且支持 MADV_REMOVE 回收与重写，无返回值
#[test]
fn unsealed_shm_fd_omits_sealing_and_supports_madv_remove() {
    let fd = create_unsealed_shm_fd().expect("unsealed shm fd");
    let raw_fd = fd.as_raw_fd();

    // 1. 确认开启 FD_CLOEXEC 且不含 F_SEAL_SHRINK（无 MFD_ALLOW_SEALING 时内核仅设 F_SEAL_SEAL 并永久拒绝追加 F_SEAL_SHRINK）
    let fd_flags = unsafe { nix::libc::fcntl(raw_fd, nix::libc::F_GETFD) };
    assert!(fd_flags >= 0);
    assert_ne!(fd_flags & nix::libc::FD_CLOEXEC, 0);

    let seals = unsafe { nix::libc::fcntl(raw_fd, nix::libc::F_GET_SEALS) };
    if seals >= 0 {
        assert_eq!(seals & nix::libc::F_SEAL_SHRINK, 0);
        assert_eq!(seals, nix::libc::F_SEAL_SEAL);
        let add_shrink =
            unsafe { nix::libc::fcntl(raw_fd, nix::libc::F_ADD_SEALS, nix::libc::F_SEAL_SHRINK) };
        assert_eq!(add_shrink, -1);
    } else {
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(nix::libc::EINVAL)
        );
    }

    // 2. 验证 MADV_REMOVE 回收中间整页后读出全零，且后续写入可正常分配新页而不破坏映射
    let page = usize::try_from(unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) }).unwrap();
    let total = page * 4;
    let file = File::from(fd);
    file.set_len(total as u64).unwrap();
    let mut mmap = unsafe { MmapMut::map_mut(&file).unwrap() };
    mmap.fill(0xAB);

    let rc = unsafe {
        nix::libc::madvise(
            mmap.as_mut_ptr().add(page).cast(),
            page * 2,
            nix::libc::MADV_REMOVE,
        )
    };
    assert_eq!(rc, 0);
    assert!(mmap[..page].iter().all(|&byte| byte == 0xAB));
    assert!(mmap[page..page * 3].iter().all(|&byte| byte == 0));
    assert!(mmap[page * 3..].iter().all(|&byte| byte == 0xAB));

    mmap[page..page * 2].fill(0x5A);
    assert!(mmap[page..page * 2].iter().all(|&byte| byte == 0x5A));
}

/// 【共享内存池测试】【空闲表合并】无参数，验证乱序释放的槽位区间能够按偏移合并回单一连续区间，无返回值
#[test]
fn freelist_allocates_coalesces_and_reclaims_adjacent_blocks() {
    let mut free = vec![FreelistEntry {
        offset: 0,
        len: 512,
    }];
    let a = take_from_freelist(&mut free, 128).unwrap();
    let b = take_from_freelist(&mut free, 128).unwrap();
    let c = take_from_freelist(&mut free, 128).unwrap();
    let d = take_from_freelist(&mut free, 128).unwrap();
    assert_eq!((a, b, c, d), (0, 128, 256, 384));
    assert!(free.is_empty());
    assert_eq!(take_from_freelist(&mut free, 64), None);

    // 1. 先归还不相邻块形成两个独立空闲岛，再归还中间与尾部块验证完整合并
    coalesce_into_freelist(&mut free, a, 128);
    coalesce_into_freelist(&mut free, c, 128);
    assert_eq!(
        free,
        vec![
            FreelistEntry {
                offset: 0,
                len: 128
            },
            FreelistEntry {
                offset: 256,
                len: 128
            },
        ]
    );

    coalesce_into_freelist(&mut free, b, 128);
    coalesce_into_freelist(&mut free, d, 128);
    assert_eq!(
        free,
        vec![FreelistEntry {
            offset: 0,
            len: 512
        }]
    );
}

/// 【共享内存池测试】【尺寸边界】无参数，验证零字节与超过协议 32 位上限的容量被拒绝，无返回值
#[test]
fn protocol_pool_size_rejects_zero_and_i32_overflow() {
    assert_eq!(
        protocol_pool_size(0).unwrap_err().kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        protocol_pool_size(i32::MAX as usize + 1)
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(protocol_pool_size(4096).unwrap(), 4096);
}
