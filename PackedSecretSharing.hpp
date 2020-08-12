#pragma once

#include <NTL/lzz_p.h>
#include <NTL/lzz_pX.h>
#include <NTL/vector.h>
#include <memory>

#include "OddFFT/OddFFT.hpp"
#include "PRG/PRG.hpp"

namespace leviosa {

// TODO make convention on function naming

// This class implements packed secret sharing over Z_p for some prime p.
// It uses discrete Fast Fourier Transforms to achieve this in time O(n log(n)),
// where n is the number of points that are being shared/reconstructed. The
// constructor takes as input the number of secrets and shares, the degree of
// the polynomial to be used for sharing, the prime p, and a root of unity w.
// Secrets are ``stored`` in odd powers of this root (through a custom variant
// of FFT), and shares in even powers (through standard FFT as provided by NTL).
// This class requires that the number of shares is a power of 2.
//
// In addition to the sharing and reconstruction functions, this class
// implements a few other functions that are more specifici to the leviosa
// protocol.
class PackedSecretSharing {
 public:
  PackedSecretSharing(int _num_secrets, int _num_shares, int _degree,
                      long _prime, NTL::zz_p _root, PRG& _prg);

  // share takes as input a vector of secrets and encodes it into a vector of
  // shares using packed secret sharing. The polynomial used to persorm this
  // sharing can be retrieved by passing an optional pointer to the function.
  void share(const NTL::Vec<NTL::zz_p>& secrets, NTL::Vec<NTL::zz_p>& shares,
             NTL::zz_pX* poly = nullptr);

  // deterministic_share is analogous to share, but in this case the polynomial
  // used for the secret sharing is fixed, so that for any set of secrets (and
  // parameters used to construct this class) the shares returned by this
  // funciton are always the same. This is used in the leviosa protocol when
  // both parties know the secrets, so that there is no need to send the
  // encrypted shares along for the watchlist, as each party can compute
  // them locally.
  void deterministic_share(const NTL::Vec<NTL::zz_p>& secrets,
                           NTL::Vec<NTL::zz_p>& shares, NTL::zz_pX* poly);

  // reconstruct uses the given vector of shares to reconstruct a vector of
  // secrets. For efficiency reasons, not all shares are used in the
  // reconstruction process, so some of them could be inconsistent. The optional
  // flag checkAllShares ensures this is not the case, and that the underlying
  // reconstructed polynomial has degree <= max_degree.
  int reconstruct(NTL::Vec<NTL::zz_p>& secrets,
                  const NTL::Vec<NTL::zz_p>& shares, NTL::zz_pX* poly = nullptr,
                  bool checkAllShares = false, int max_degree = 0);

  // int reconstructAndCheck(int max_degree, NTL::Vec<NTL::zz_p>& secrets,
  //                         const NTL::Vec<NTL::zz_p>& shares,
  //                         NTL::zz_pX* poly = nullptr);

  // generateRandomPoly generates a uniformly random polynomial of degree
  // at most this.degree, and assigns such polynomial and the secrets and shares
  // derived from evaluating it to the objects at the provided addresses.
  void generateRandomPoly(NTL::Vec<NTL::zz_p>* secrets = nullptr,
                          NTL::Vec<NTL::zz_p>* shares = nullptr,
                          NTL::zz_pX* poly = nullptr);

  // generateRandomPolyDoubleDegreeZeroSum generates a uniformly random
  // polynomial of degree at most 2*this.degree, subject to the constraint the
  // sum of the secrets derived from this polynomial is zero. It then assigns
  // such polynomial and the secrets and shares derived from evaluating it to
  // the objects at the provided addresses.
  void generateRandomPolyDoubleDegreeZeroSum(
      NTL::Vec<NTL::zz_p>* secrets = nullptr,
      NTL::Vec<NTL::zz_p>* shares = nullptr, NTL::zz_pX* poly = nullptr);

  // generateRandomPolyDoubleDegreeZeroOnSecrets generates a uniformly random
  // polynomial of degree at most 2*this.degree, subject to the constraint that
  // it evaluates to zero on all the secret points. It then assigns such
  // polynomial and the secrets and shares derived from evaluating it to the
  // objects at the provided addresses.
  void generateRandomPolyDoubleDegreeZeroOnSecrets(
      NTL::Vec<NTL::zz_p>* shares = nullptr, NTL::zz_pX* poly = nullptr);

  // Checks that the values of the polynomial are consistent with the ones in
  // the expected_shares map.
  bool checkConsistency(const NTL::zz_pX& poly,
                        const std::map<long, NTL::zz_p>& expected_shares);

  // Checks that the polynomial has degree at most 2*this.degree, and that the
  // values of the polynomial in the secret points are zero.
  bool checkZeroOnSecretsDoubleDegree(const NTL::zz_pX& poly);

 private:
  int num_secrets;
  int num_shares;
  int degree;
  long prime;
  NTL::zz_p root;
  std::vector<long> rootTable;
  std::vector<NTL::mulmod_precon_t> precondTable;

  long order;  // order of the root
  long log_order;
  PRG& prg;

  NTL::Vec<NTL::zz_p> secret_points;  // This represents the points where the
                                      // secrets are stored.

  NTL::FFTPrimeInfo* fft_info;

  void polyToShares(NTL::zz_pX& poly, NTL::Vec<NTL::zz_p>& shares);

  NTL::zz_pX zeroOnSecrets;
};

}  // namespace leviosa