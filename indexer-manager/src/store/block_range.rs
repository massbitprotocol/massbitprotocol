use chain_solana::types::{BlockPtr, BlockSlot};
use derive_more::Constructor;
use diesel::pg::Pg;
use diesel::query_builder::{AstPass, QueryFragment};
use diesel::result::QueryResult;
///! Utilities to deal with block numbers and block ranges
use diesel::serialize::{Output, ToSql};
use diesel::sql_types::{Integer, Range};
use indexer_orm::DieselBlockSlot;
use massbit_data::store::chain::BLOCK_NUMBER_MAX;
use massbit_storage_postgres::relational::Table;
use std::io::Write;
use std::ops::{Bound, RangeBounds, RangeFrom};

/// The name of the column in which we store the block range
pub(crate) const BLOCK_RANGE_COLUMN: &str = "block_range";

/// The SQL clause we use to check that an entity version is current;
/// that version has an unbounded block range, but checking for
/// `upper_inf(block_range)` is slow and can't use the exclusion
/// index we have on entity tables; we therefore check if i32::MAX is
/// in the range
pub(crate) const BLOCK_RANGE_CURRENT: &str = "block_range @> 9223372036854775807";

/// Most indexer metadata entities are not versioned. For such entities, we
/// want two things:
///   - any CRUD operation modifies such an entity in place
///   - queries by a block number consider such an entity as present for
///     any block number
/// We therefore mark such entities with a block range `[-1,\infinity)`; we
/// use `-1` as the lower bound to make it easier to identify such entities
/// for troubleshooting/debugging
pub(crate) const BLOCK_UNVERSIONED: BlockSlot = -1;

pub(crate) const UNVERSIONED_RANGE: (Bound<BlockSlot>, Bound<BlockSlot>) =
    (Bound::Included(BLOCK_UNVERSIONED), Bound::Unbounded);

/// The range of blocks for which an entity is valid. We need this struct
/// to bind ranges into Diesel queries.
#[derive(Clone, Debug)]
pub struct BlockRange(Bound<BlockSlot>, Bound<BlockSlot>);

// Doing this properly by implementing Clone for Bound is currently
// a nightly-only feature, so we need to work around that
fn clone_bound(bound: Bound<&BlockSlot>) -> Bound<BlockSlot> {
    match bound {
        Bound::Included(nr) => Bound::Included(*nr),
        Bound::Excluded(nr) => Bound::Excluded(*nr),
        Bound::Unbounded => Bound::Unbounded,
    }
}

/// Return the block number contained in the history event. If it is
/// `None` panic because that indicates that we want to perform an
/// operation that does not record history, which should not happen
/// with how we currently use relational schemas
pub(crate) fn block_number(block_ptr: &BlockPtr) -> BlockSlot {
    block_ptr.number
}

/// Generate the clause that checks whether `block` is in the block range
/// of an entity
#[derive(Constructor)]
pub struct BlockRangeContainsClause<'a> {
    table: &'a Table,
    table_prefix: &'a str,
    block: BlockSlot,
}

impl<'a> QueryFragment<Pg> for BlockRangeContainsClause<'a> {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();

        out.push_sql(self.table_prefix);
        out.push_identifier(BLOCK_RANGE_COLUMN)?;
        out.push_sql(" @> ");
        out.push_bind_param::<DieselBlockSlot, _>(&self.block)?;
        if self.table.is_account_like && self.block < BLOCK_NUMBER_MAX {
            // When block is BLOCK_NUMBER_MAX, these checks would be wrong; we
            // don't worry about adding the equivalent in that case since
            // we generally only see BLOCK_NUMBER_MAX here for metadata
            // queries where block ranges don't matter anyway
            out.push_sql(" and coalesce(upper(");
            out.push_identifier(BLOCK_RANGE_COLUMN)?;
            out.push_sql("), 9223372036854775807) > ");
            out.push_bind_param::<DieselBlockSlot, _>(&self.block)?;
            out.push_sql(" and lower(");
            out.push_identifier(BLOCK_RANGE_COLUMN)?;
            out.push_sql(") <= ");
            out.push_bind_param::<DieselBlockSlot, _>(&self.block)
        } else {
            Ok(())
        }
    }
}

impl From<RangeFrom<BlockSlot>> for BlockRange {
    fn from(range: RangeFrom<BlockSlot>) -> BlockRange {
        BlockRange(
            clone_bound(range.start_bound()),
            clone_bound(range.end_bound()),
        )
    }
}

impl ToSql<Range<DieselBlockSlot>, Pg> for BlockRange {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> diesel::serialize::Result {
        let pair = (self.0, self.1);
        ToSql::<Range<DieselBlockSlot>, Pg>::to_sql(&pair, out)
    }
}
