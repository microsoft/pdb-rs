using Microsoft.VisualStudio.TestPlatform;
using Pdb;

namespace PdbTest {
    [TestClass]
    public sealed class Test1 {
        [TestMethod]
        public void TestMethod1() {
            using var f = File.OpenRead("c:\\pdb\\pdbtool.pdb");
            using var p = MsfReader.Open(f);
            Console.WriteLine($"Number of streams: {p.NumStreams}");
        }
    }
}
