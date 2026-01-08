use sha2::{Digest, Sha256};

use crate::{MerkleNode, ScriptTree};

/// 合并两个哈希值
pub fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    let mut result = [0u8; 32];
    result.copy_from_slice(&hasher.finalize());
    result
}

/// 单个数据的哈希值
pub fn hash_single(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let mut result = [0u8; 32];
    result.copy_from_slice(&hasher.finalize());
    result
}

#[derive(Clone, Debug)]
pub struct MerkleProof {
    // 叶子数据
    pub leaf: Vec<u8>,
    // 从叶子到根的证明路径
    // 每个元素是 (sibling_hash, is_right)
    // is_right = true 表示当前节点是右子，sibling 在左
    pub path: Vec<(Box<[u8; 32]>, bool)>,
}

impl MerkleProof {
    // 验证证明是否有效
    pub fn verify(&self, root_hash: &[u8; 32]) -> bool {
        let mut current = hash_single(&self.leaf);

        for (sibling, is_right) in &self.path {
            let sibling_hash: &[u8; 32] = sibling;
            current = if *is_right {
                // 当前节点是右子，sibling 在左
                hash_pair(sibling_hash, &current)
            } else {
                // 当前节点是左子，sibling 在右
                hash_pair(&current, sibling_hash)
            };
        }

        &current == root_hash
    }

    // 计算包含证明中所有哈希的成本（字节数）
    pub fn proof_size(&self) -> usize {
        32 * self.path.len() // 每个证明元素 32 字节
    }
}

pub fn generate_proof(tree: &ScriptTree, leaf_index: usize) -> Option<MerkleProof> {
    if leaf_index >= tree.leaves.len() {
        return None;
    }

    let leaf_data = tree.leaves[leaf_index].script_data.clone();
    let mut path = Vec::new();

    // 遍历树，收集证明路径
    collect_proof_path(&tree.root, leaf_index, tree.leaves.len(), &mut path);

    Some(MerkleProof {
        leaf: leaf_data,
        path,
    })
}

// 计算左子树在完全二叉树中应有的叶子数
fn left_child_size(total_leaves: usize) -> usize {
    if total_leaves <= 1 {
        return total_leaves;
    }
    let mut size = 1;
    while size * 2 < total_leaves {
        size *= 2;
    }
    size
}

fn collect_proof_path(
    node: &MerkleNode,
    target_index: usize,
    total_leaves: usize,
    path: &mut Vec<(Box<[u8; 32]>, bool)>,
) -> bool {
    match node {
        MerkleNode::Leaf(_) => target_index == 0,
        MerkleNode::Branch { left, right } => {
            // 计算左子树应该有多少个叶子
            let left_size = left_child_size(total_leaves);

            if target_index < left_size {
                // 目标在左子树
                if collect_proof_path(left, target_index, left_size, path) {
                    // 加入右兄弟作为证明
                    let sibling_hash = right.hash();
                    path.push((Box::new(sibling_hash), true));
                    return true;
                }
            } else {
                // 目标在右子树
                let right_size = total_leaves - left_size;
                if collect_proof_path(right, target_index - left_size, right_size, path) {
                    // 加入左兄弟作为证明
                    let sibling_hash = left.hash();
                    path.push((Box::new(sibling_hash), false));
                    return true;
                }
            }
            false
        }
    }
}
