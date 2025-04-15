use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers;

pub(crate) fn duplicate<Value: Clone + Send + 'static, Error: Clone + Send + 'static>(
    mut stream: Pin<Box<impl Stream<Item = Result<Value, Error>> + Send + ?Sized + 'static>>,
    buffer_size: usize,
) -> (
    impl Stream<Item = Result<Value, Error>>,
    impl Stream<Item = Result<Value, Error>>,
) {
    let (first_transmit, first_receive) = mpsc::channel(buffer_size);
    let (second_transmit, second_receive) = mpsc::channel(buffer_size);

    tokio::spawn(async move {
        let mut is_first_ok = true;
        let mut is_second_ok = true;
        while let Some(item) = stream.next().await {
            if is_first_ok {
                is_first_ok = first_transmit.send(item.clone()).await.is_ok();
            }
            if is_second_ok {
                is_second_ok = second_transmit.send(item.clone()).await.is_ok();
            }
        }
    });

    (
        wrappers::ReceiverStream::new(first_receive),
        wrappers::ReceiverStream::new(second_receive),
    )
}
