// Verify the given prehashed message using ECDSA.
///
/// This trait is intended to be implemented on type which can access
/// the affine point represeting the public key via `&self`, such as a
/// particular curve's `AffinePoint` type.
#[cfg(feature = "arithmetic")]
#[cfg_attr(docsrs, doc(cfg(feature = "arithmetic")))]
pub trait VerifyPrimitive<C>
where
    C: Curve + ProjectiveArithmetic,
    FieldBytes<C>: From<Scalar<C>> + for<'r> From<&'r Scalar<C>>,
    Scalar<C>: PrimeField<Repr = FieldBytes<C>>,
    SignatureSize<C>: ArrayLength<u8>,
{
    /// Verify the prehashed message against the provided signature
    ///
    /// Accepts the following arguments:
    ///
    /// - `verify_key`: public key to verify the signature against
    /// - `hashed_msg`: prehashed message to be verified
    /// - `signature`: signature to be verified against the key and message
    fn verify_prehashed(
        &self,
        hashed_msg: &Scalar<C>,
        signature: &Signature<C>,
    ) -> Result<(), Error>;
}

pub trait Digest {
    type OutputSize: ArrayLength<u8>;
    fn new() -> Self;
    fn update(&mut self, data: impl AsRef<[u8]>);
    fn chain(self, data: impl AsRef<[u8]>) -> Self;
    fn finalize(self) -> GenericArray<u8, Self::OutputSize>;
    fn finalize_reset(&mut self) -> GenericArray<u8, Self::OutputSize>;
    fn reset(&mut self);
    fn output_size() -> usize;
    fn digest(data: &[u8]) -> GenericArray<u8, Self::OutputSize>;
}

/*
Keep Alive
    send packet constatnly
    flags in zmq_keep_alive
    30s interval
Linger -- how long after the main thread ends
    bulky information
    zmq_linger

Thread
    ROUTER 1mb/s 

high water mark
    
bottle of of CPU processing
    relay mechanism
    spawn workers vs spawn threads manually

Compare conversion throughput vs msessageing throughput
    benchmark serialization

poll API
    change socket into non-blocking


*/

考虑问题及测试方案：

    1. 多人向socket发消息或者package很大，zmq如何queue？怎么spawn threads？
        how does high water mark work?
        (让zmq自己take care线程)
        http://api.zeromq.org/2-1:zmq-setsockopt
        - 试一下多人轰炸，或者巨大package，print出package loss
        - 看它自己的test modual
    
    2. 具体关于线程，spawn worker可能是首选
        有没有需要verify很多人authentication的情况？
        - 测试ECDSA的runtime
        - 人多时采取spawn worker同时做authentication

    3. blocking: 如果server.recv(identity)没收到Alice的信息，它会一直等
        - 用poll，它就poll一下没有就算了
        http://api.zeromq.org/2-1:zmq-poll
        
    4. Keep Alive: 需不需要一直保持sender和Alice的connection？如何keep alive?
        - 先不管
    5. Linger: server main thread 结束后如果有人继续发消息需要handle。
        - 先不管











