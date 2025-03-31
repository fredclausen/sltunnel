use tokio::io::{copy, split, ReadHalf, Result, WriteHalf};

use crate::traits::{Readable, Writable};

pub struct Pipe<R, W>
where
    R: Readable,
    W: Writable,
{
    reader: R,
    writer: W,
}

impl<U, D> Pipe<ReadHalf<U>, WriteHalf<D>>
where
    U: Readable + Writable,
    D: Readable + Writable,
{
    pub async fn run(&mut self) -> Result<u64> {
        copy::<ReadHalf<U>, WriteHalf<D>>(&mut self.reader, &mut self.writer).await
    }
}

pub(crate) type Pipes<U, D> = (
    Pipe<ReadHalf<U>, WriteHalf<D>>,
    Pipe<ReadHalf<D>, WriteHalf<U>>,
);

type PipePair<U, D> = (
    Pipe<ReadHalf<U>, WriteHalf<D>>,
    Pipe<ReadHalf<D>, WriteHalf<U>>,
);

pub(crate) fn pipes<U, D>(upstream: U, downstream: D) -> PipePair<U, D>
where
    U: Readable + Writable,
    D: Readable + Writable,
{
    let (upstream_read, upstream_write) = split(upstream);
    let (downstream_read, downstream_write) = split(downstream);

    (
        Pipe {
            reader: upstream_read,
            writer: downstream_write,
        },
        Pipe {
            reader: downstream_read,
            writer: upstream_write,
        },
    )
}
