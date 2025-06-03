
namespace Pdb;

static class MsfUtils {
    internal static int NumPagesForBytes(uint numBytes, int pageSizeShift) {
        uint lowMask = (1u << pageSizeShift) - 1u;
        int carry = (numBytes & lowMask) != 0 ? 1 : 0;
        return (int)(numBytes >> pageSizeShift) + carry;
    }

    internal static bool IsPowerOf2(uint n) {
        return n > 0 && ((n & (n - 1u)) == 0u);
    }

    internal static int GetLog2OfPageSize(uint n) {
        if (n == 0) {
            throw new Exception("The page size is invalid because it is zero.");
        }

        int s = 0;
        while (n > 1) {
            ++s;
            n >>= 1;
        }

        if (n != 1) {
            throw new Exception("The page size is invalid because it is not a power of 2.");
        }

        return s;

    }

    internal static uint AlignDownToPageSize(uint value, int pageSizeShift) {
        uint mask = ~((1u << pageSizeShift) - 1u);
        return value & mask;
    }

    internal static bool SpanEq(in Span<byte> a, in Span<byte> b) {
        if (a.Length != b.Length) {
            return false;
        }
        for (int i = 0; i < a.Length; ++i) {
            if (a[i] != b[i]) {
                return false;
            }
        }
        return true;
    }
}
