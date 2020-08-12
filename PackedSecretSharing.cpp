#include <iostream>

#include <NTL/FFT_impl.h>
#include <NTL/lzz_p.h>

//#define DEBUG 1

#include <assert.h>
#include <Util.hpp>
#include "OddFFT/OddFFT.hpp"
#include "PackedSecretSharing.hpp"

#include "pthread.h"

namespace leviosa {
using NTL::Vec;
using NTL::zz_p;
using NTL::zz_pX;

// reverse_index returns the number obtained by reversing the first bit_length
// bits of v. For example, reverse_bits(5,4) = 10, as in (4 bits) binary 5 =
// 0101, which is reversed to 8 = 1010. It assumes bit_length is > 0.
unsigned int reverse_index(unsigned int v, int bit_length) {
  // Adapted from
  // http://graphics.stanford.edu/~seander/bithacks.html#BitReverseObvious
  unsigned int r = 0;  // r will contain the reversed bits of v;

  for (; v; v >>= 1) {
    r <<= 1;
    r |= v & 1;
    bit_length--;
  }
  r <<= bit_length;  // shift when v's highest bits are zero
  return r;
}

PackedSecretSharing::PackedSecretSharing(int _num_secrets, int _num_shares,
                                         int _degree, long _prime, zz_p _root,
                                         PRG& _prg)
    : num_secrets(_num_secrets),
      num_shares(_num_shares),
      degree(_degree),
      prime(_prime),
      root(_root),
      prg(_prg) {
  // TODO check these assumptions,
  // TODO Change the assertions into exceptions
  // assert(is_power_of_2(num_shares));
  // The factor two is to make sure we can reconstruct the polynomial even after
  // one pointwise multiplication.

  assert(((degree + 1) % (1 << NTL_FFT_RDUP)) == 0);

  assert(degree * 2 + 1 <= num_shares);

  assert(is_power_of_2(degree + 1));

  // if (NTL::zz_pInfo == NULL || NTL::zz_pInfo->p != prime) {
  //   NTL::zz_p::UserFFTInit(prime);
  // }
  // fft_info = NTL::zz_pInfo->p_info;
  NTL::zz_p::init(prime);
  fft_info = new NTL::FFTPrimeInfo();
  InitFFTPrimeInfo(*fft_info, prime, NTL::rep(root), 0);

  // 这他妈在干嘛？？
  // 底下那个 do loop 怎么看 rootTable 最后多长？ 是循环到 power % P = 1 为止吗？
  long power = 1;
  rootTable.push_back(power);
  precondTable.push_back(NTL::PrepMulModPrecon(1, prime));
  NTL::mulmod_precon_t prec = NTL::PrepMulModPrecon(root._zz_p__rep, prime);
  do {
    power = NTL::MulModPrecon(power, root._zz_p__rep, prime, prec);
    precondTable.push_back(NTL::PrepMulModPrecon(power, prime));
    rootTable.push_back(power);
  } while (power != 1);

  order = rootTable.size() - 1;
  log_order = log2(order);

  if (!is_power_of_2(order)) {
    throw std::invalid_argument("the root order must be a power of two");
  }

  // order是整个rootTable的长度，也就是所有可以用的x值
  // 可用的x/2数量必须大于 2 * degree + 1 = num_secret + num shares
  // 如果算了512个omegas, degree差不多就是128
  assert(order / 2 >= 2 * degree + 1);

  secret_points.SetLength(num_secrets);
  for (int i = 0; i < num_secrets; i++) {
    secret_points[i] = rootTable[reverse_index(i + (order / 2), log_order)];
  }
}

void PackedSecretSharing::share(const Vec<zz_p>& secrets, Vec<zz_p>& shares,
                                zz_pX* poly) {
  assert(secrets.length() == num_secrets);

  // buf is only used as a temporary object if poly is not given.
  zz_pX buf;
  if (poly == nullptr) {
    poly = &buf;
  }
  // size 是 secret + randomness
  long size = degree + 1;

  // Create a vector that contains all the secrets followed by random zz_p
  // elements for a total of 2^log_odd_fft_size coefficients
  Vec<zz_p> images;
  images.SetLength(size);
  for (int i = 0; i < num_secrets; i++) {
    images[i] = secrets[i];
  }
  Vec<zz_p> random_coeff = prg.random_vec_zz_p(size - num_secrets);
  // Note: degree + 1 = 2^log_odd_fft_size
  for (int i = num_secrets; i < size; i++) {
    images[i] = random_coeff[i - num_secrets];
  }
  //所以这个poly的长度确实是 secret + randomness
  //假设是256
  poly->SetLength(size);
  //底下这个 log_order就很奇怪了，order是omegas的数量（如512，log(512) = 9) 不知道丢进去干嘛
  //这里一定包含着 secret_points 因为你必须要pairs
  NTL::new_ifft((long*)poly->rep.begin(), (long*)images.begin(), log_order,
                *fft_info, size);
  for (int i = 0; i < size; i++) {
    poly->rep[i] =
        NTL::MulModPrecon(poly->rep[i]._zz_p__rep, rootTable[order - i], prime,
                          precondTable[order - i]);
  }
  //反正就当它搞了一个D = 256 的poly
  polyToShares(*poly, shares);
}

void PackedSecretSharing::polyToShares(zz_pX& poly,
                                       NTL::Vec<NTL::zz_p>& shares) {
  int size;
  if (num_shares > order / 2) {
    size = NTL::FFTRoundUp(num_shares + num_secrets, log_order);
  } else {
    size = NTL::FFTRoundUp(num_shares, log_order);
  }

  //等下这个poly因该是256。。。但他resize了 
  //唯一make sense的是 shares size = poly size

  //  long poly_deg = deg(poly);
  poly.SetLength(size);
  shares.SetLength(size);
  // // TODO investigate if this zeroing is necessary
  // for (int i = poly_deg + 1; i < (1L << log_even_fft_size_fwd); i++) {
  //   poly[i] = 0;
  // }

  // TODO maybe one of the sizes can be shortened.
  NTL::new_fft((long*)shares.begin(), (long*)poly.rep.begin(), log_order,
               *fft_info, size, size);

  poly.normalize();

  if (num_shares > order / 2) {
    for (int i = 0; i < num_shares - (order / 2); i++) {
      shares[(order / 2) + i] = shares[(order / 2) + i + num_secrets];
    }
  }

  shares.SetLength(num_shares);
}

int PackedSecretSharing::reconstruct(Vec<zz_p>& secrets,
                                     const Vec<zz_p>& shares, zz_pX* poly,
                                     bool checkAllShares, int max_degree) {
  if (shares.length() != num_shares) {
    throw std::logic_error("PackedSecretSharing::reconstruct called with " +
                           std::to_string(shares.length()) + " shares (" +
                           std::to_string(num_shares) + " expected).");
  }

  // buf is only used as a temporary object if poly is not given.
  zz_pX buf;
  if (poly == nullptr) {
    poly = &buf;
  }

  int size = 2 * (degree + 1);

  poly->SetLength(size);

  // Note that this implicitly considers only the first size
  // shares, ignoring the rest.
  NTL::new_ifft((long*)poly->rep.begin(), (long*)shares.begin(), log_order,
                *fft_info, size);

  poly->normalize();  // Necessary after manipulating the internal
                      // representation.

  DEBUG_PRINT(*poly)
  if (checkAllShares) {
    if (deg(*poly) > max_degree) {
      throw std::runtime_error(
          "reconstruct failed because the polynomial's degree (" +
          std::to_string(deg(*poly)) + ") is greater than max_degree (" +
          std::to_string(max_degree) + ")");
    }

    // Check that all the shares match
    Vec<zz_p> shares_for_checking;
    // TODO maybe we can be clever and not recompute all the shares, but only
    // the ones we did not use to reconstruct.
    polyToShares(*poly, shares_for_checking);
    for (int i = size; i < num_shares; i++) {
      if (shares[i] != shares_for_checking[i]) {
        throw std::runtime_error(
            "reconstruct failed because not all shares are consistent with the "
            "polynomial");
      }
    }
  }

  // Depending on the degree of the reconstructed polynomial, we decide which
  // size we need for the transform. Doing a smaller one saves time, but both
  // should be correct.
  if (deg(*poly) <= degree) {
    size = degree + 1;
  } else {
    size = 2 * (degree + 1);
  }

  secrets.SetLength(size);
  poly->SetLength(size);

  NTL::zz_pX shiftedPoly;
  shiftedPoly.SetLength(size);
  for (int i = 0; i < size; i++) {
    shiftedPoly.rep[i] = NTL::MulModPrecon(
        poly->rep[i]._zz_p__rep, rootTable[i], prime, precondTable[i]);
  }
  NTL::new_fft((long*)secrets.begin(), (long*)shiftedPoly.rep.begin(),
               log_order, *fft_info, size, size);

  secrets.SetLength(num_secrets);

  poly->normalize();  // Necessary after manipulating the internal
                      // representation.

  return deg(*poly);
}

void PackedSecretSharing::generateRandomPoly(Vec<zz_p>* secrets,
                                             Vec<zz_p>* shares, zz_pX* poly) {
  // buf is only used as a temporary object if poly is not given.
  zz_pX buf;
  if (poly == nullptr) {
    poly = &buf;
  }

  Vec<zz_p> coeff = prg.random_vec_zz_p(degree + 1);

  // TODO investigate batch copy
  poly->SetLength(degree + 1);
  for (int i = 0; i < degree + 1; i++) {
    poly->rep[i] = coeff[i];
  }

  if (shares != nullptr) {
    polyToShares(*poly, *shares);
  }

  long size = degree + 1;

  if (secrets != nullptr) {
    secrets->SetLength(size);

    NTL::zz_pX shiftedPoly;
    shiftedPoly.SetLength(size);
    for (int i = 0; i < size; i++) {
      shiftedPoly.rep[i] = NTL::MulModPrecon(
          poly->rep[i]._zz_p__rep, rootTable[i], prime, precondTable[i]);
    }
    NTL::new_fft((long*)secrets->begin(), (long*)shiftedPoly.rep.begin(),
                 log_order, *fft_info, size, size);
    secrets->SetLength(num_secrets);

    DEBUG_PRINT(secrets)
  }
  poly->normalize();
}

// Note: this function is not profiled as it uses NTL non-fft interpolation, but
// it is never called in the online phase.
void PackedSecretSharing::generateRandomPolyDoubleDegreeZeroSum(
    Vec<zz_p>* secrets, Vec<zz_p>* shares, zz_pX* poly) {
  // buf is only used as a temporary object if poly is not given.
  zz_pX buf;
  if (poly == nullptr) {
    poly = &buf;
  }

  int size = 2 * (degree + 1);

  Vec<zz_p> ext_secrets = prg.random_vec_zz_p(size);
  NTL::zz_p sum(0);

  for (int i = 0; i < num_secrets - 1; i++) {
    sum += ext_secrets[i];
  }

  ext_secrets[num_secrets - 1] = -sum;

  poly->SetLength(size);
  NTL::new_ifft((long*)poly->rep.begin(), (long*)ext_secrets.begin(), log_order,
                *fft_info, size);
  for (int i = 0; i < size; i++) {
    poly->rep[i] =
        NTL::MulModPrecon(poly->rep[i]._zz_p__rep, rootTable[order - i], prime,
                          precondTable[order - i]);
  }

  poly->normalize();

  // now we lower the degree of the polynomial while preserving the values on
  // the secret points. This whole process preserves uniformity as it can be
  // seen as a projection of a uniformly sampled polynomial to a vector subspace
  // of the space it was sampled from
  if (NTL::IsZero(zeroOnSecrets)) {
    NTL::BuildFromRoots(zeroOnSecrets, secret_points);
  }

  while (deg(*poly) > 2 * degree) {
    *poly -= (zeroOnSecrets << (deg(*poly) - num_secrets)) * LeadCoeff(*poly);
  }

  DEBUG_PRINT(*poly)
  DEBUG_PRINT(deg(*poly))

  if (shares != nullptr) {
    polyToShares(*poly, *shares);
  }
}

void PackedSecretSharing::generateRandomPolyDoubleDegreeZeroOnSecrets(
    Vec<zz_p>* shares, zz_pX* poly) {
  // buf is only used as a temporary object if poly is not given.
  zz_pX buf;
  if (poly == nullptr) {
    poly = &buf;
  }

  int size = 2 * (degree + 1);

  Vec<zz_p> ext_secrets;
  ext_secrets.SetLength(size);
  zz_p sum;
  sum = 0;
  // The first num_secret points are 0 as they represent the values of the
  // polynomial on the secret points
  for (int i = num_secrets; i < ext_secrets.length(); i++) {
    // The vector should be zero on secret points (the first num_secrets),
    // random on the rest.
    ext_secrets[i] = prg.next_zz_p();
  }

  poly->SetLength(size);
  NTL::new_ifft((long*)poly->rep.begin(), (long*)ext_secrets.begin(), log_order,
                *fft_info, size);
  for (int i = 0; i < size; i++) {
    poly->rep[i] =
        NTL::MulModPrecon(poly->rep[i]._zz_p__rep, rootTable[order - i], prime,
                          precondTable[order - i]);
  }

  poly->normalize();

  // now we lower the degree of the polynomial while preserving the values on
  // the secret points. This whole process preserves uniformity as it can be
  // seen as a projection of a uniformly sampled polynomial to a vector subspace
  // of the space it was sampled from
  if (NTL::IsZero(zeroOnSecrets)) {
    NTL::BuildFromRoots(zeroOnSecrets, secret_points);
  }

  while (deg(*poly) > 2 * degree) {
    *poly -= (zeroOnSecrets << (deg(*poly) - num_secrets)) * LeadCoeff(*poly);
  }

  if (shares != nullptr) {
    polyToShares(*poly, *shares);
  }
}

bool PackedSecretSharing::checkConsistency(
    const zz_pX& poly, const std::map<long, zz_p>& expected_shares) {
  zz_pX p(poly);

  // The polynomial has degree at most degree, but we need the extra zeroes in
  // the vector of coefficients to run the FFT
  Vec<zz_p> shares;
  polyToShares(p, shares);

  for (auto& x : expected_shares) {
    if (shares[x.first] != x.second) return false;
  }

  return true;
}

bool PackedSecretSharing::checkZeroOnSecretsDoubleDegree(const zz_pX& poly) {
  int size = 2 * (degree + 1);

  Vec<zz_p> secrets;
  secrets.SetLength(size);

  NTL::zz_pX shiftedPoly;
  shiftedPoly.SetLength(size);
  for (int i = 0; i < size; i++) {
    shiftedPoly.rep[i] = NTL::MulModPrecon(poly.rep[i]._zz_p__rep, rootTable[i],
                                           prime, precondTable[i]);
  }
  NTL::new_fft((long*)secrets.begin(), (long*)shiftedPoly.rep.begin(),
               log_order, *fft_info, order, size);

  // The secret points should be on every other location
  for (int i = 0; i < num_secrets; i++) {
    if (secrets[i] != 0) {
      return false;
    }
  }

  return true;
}

void PackedSecretSharing::deterministic_share(const Vec<zz_p>& secrets,
                                              Vec<zz_p>& shares, zz_pX* poly) {
  assert(secrets.length() == num_secrets);

  // buf is only used as a temporary object if poly is not given.
  zz_pX buf;
  if (poly == nullptr) {
    poly = &buf;
  }

  // Here we might be able to use a smaller size obtained through FFTRoundUp
  int size = degree + 1;

  // Create a vector that contains all the secrets followed by enough 0s so that
  // the polynomial is fully determined
  Vec<zz_p> images;
  images.SetLength(size);
  for (int i = 0; i < num_secrets; i++) {
    images[i] = secrets[i];
  }
  for (int i = num_secrets; i < size; i++) {
    images[i] = 0;
  }

  poly->SetLength(degree + 1);
  NTL::new_ifft((long*)poly->rep.begin(), (long*)images.begin(), log_order,
                *fft_info, size);
  for (int i = 0; i < size; i++) {
    poly->rep[i] =
        NTL::MulModPrecon(poly->rep[i]._zz_p__rep, rootTable[order - i], prime,
                          precondTable[order - i]);
  }

  poly->normalize();

  DEBUG_PRINT(*poly)
  polyToShares(*poly, shares);
}

}  // namespace leviosa