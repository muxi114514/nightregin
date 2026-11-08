<#
  FromSoftware 加密归档（bhd + bdt）的读取类，extract_vanilla.ps1（黑夜君临）与 ga_er_fx.ps1、tiered_crucible.ps1（法环）共用。
  bhd 是 RSA 公钥逐块加密的 BHD5 文件表（每 256 字节解出 255 字节）；条目按路径哈希查找；bdt 里的文件部分区段是 AES-128-ECB 加密。
  Open-ErArchives：法环的公钥不内嵌，从 Smithbox 的 Andre.Formats.dll 找，逐个试 bhd 第一块能否解出 BHD5
#>
Add-Type -ReferencedAssemblies System.Numerics -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.IO;
using System.Numerics;
using System.Security.Cryptography;
using System.Text;

public static class NrArchive {
    public class Entry { public ulong Hash; public int Padded; public int Unpadded; public long Offset; public long Aes; public string Archive; }

    public static ulong Hash(string path) {
        string p = path.Trim().Replace('\\', '/').ToLowerInvariant();
        if (!p.StartsWith("/")) p = "/" + p;
        ulong h = 0;
        unchecked { foreach (char c in p) h = h * 0x85 + c; }
        return h;
    }

    static BigInteger Big(byte[] be, int off, int len) {
        var le = new byte[len + 1];
        for (int i = 0; i < len; i++) le[i] = be[off + len - 1 - i];
        return new BigInteger(le);
    }

    static byte[] Der(byte[] d, ref int pos) {
        pos++;
        int len = d[pos++];
        if ((len & 0x80) != 0) { int n = len & 0x7F; len = 0; for (int i = 0; i < n; i++) len = (len << 8) | d[pos++]; }
        var v = new byte[len]; Array.Copy(d, pos, v, 0, len); pos += len;
        return v;
    }

    // RSA 公钥逐块"解密"：m = c^e mod n，每块输出 keyBytes-1 字节（大端、左补零）
    public static byte[] DecryptBhd(string path, string pemBody) {
        byte[] der = Convert.FromBase64String(pemBody);
        int p = 0; byte[] seq = Der(der, ref p);
        int q = 0; byte[] n = Der(seq, ref q); byte[] e = Der(seq, ref q);
        BigInteger N = Big(n, 0, n.Length), E = Big(e, 0, e.Length);
        int lead = 0; while (lead < n.Length && n[lead] == 0) lead++;
        int inSize = n.Length - lead, outSize = inSize - 1;
        byte[] data = File.ReadAllBytes(path);
        var output = new byte[data.Length / inSize * outSize];
        for (int off = 0, o = 0; off + inSize <= data.Length; off += inSize, o += outSize) {
            byte[] m = BigInteger.ModPow(Big(data, off, inSize), E, N).ToByteArray();
            int len = m.Length; if (len > 0 && m[len - 1] == 0) len--;
            for (int i = 0; i < len && i < outSize; i++) output[o + outSize - 1 - i] = m[i];
        }
        return output;
    }

    // BHD5（艾尔登法环 / 黑夜君临）：桶表 + 每条 40 字节（u64 哈希、填充大小、实际大小、偏移、SHA 偏移、AES 偏移）
    public static List<Entry> Parse(byte[] b, string archive) {
        if (Encoding.ASCII.GetString(b, 0, 4) != "BHD5") throw new InvalidDataException(archive + ": not BHD5");
        int buckets = BitConverter.ToInt32(b, 16), bucketsOffset = BitConverter.ToInt32(b, 20);
        var list = new List<Entry>();
        for (int i = 0; i < buckets; i++) {
            int count = BitConverter.ToInt32(b, bucketsOffset + i * 8), off = BitConverter.ToInt32(b, bucketsOffset + i * 8 + 4);
            for (int j = 0; j < count; j++) {
                int o = off + j * 40;
                list.Add(new Entry { Hash = BitConverter.ToUInt64(b, o), Padded = BitConverter.ToInt32(b, o + 8), Unpadded = BitConverter.ToInt32(b, o + 12),
                    Offset = BitConverter.ToInt64(b, o + 16), Aes = BitConverter.ToInt64(b, o + 32), Archive = archive });
            }
        }
        return list;
    }

    public static byte[] Extract(Entry e, byte[] bhd, string bdt) {
        var buf = new byte[e.Padded];
        using (var fs = File.OpenRead(bdt)) {
            fs.Seek(e.Offset, SeekOrigin.Begin);
            int read = 0; while (read < buf.Length) { int r = fs.Read(buf, read, buf.Length - read); if (r <= 0) throw new EndOfStreamException(bdt); read += r; }
        }
        if (e.Aes != 0) {
            int a = (int)e.Aes;
            var key = new byte[16]; Array.Copy(bhd, a, key, 0, 16);
            int ranges = BitConverter.ToInt32(bhd, a + 16);
            using (var aes = Aes.Create()) {
                aes.Mode = CipherMode.ECB; aes.Padding = PaddingMode.None; aes.Key = key;
                using (var dec = aes.CreateDecryptor()) {
                    for (int r = 0; r < ranges; r++) {
                        long s = BitConverter.ToInt64(bhd, a + 20 + r * 16), t = BitConverter.ToInt64(bhd, a + 28 + r * 16);
                        if (s == -1 || t == -1 || s == t) continue;
                        dec.TransformBlock(buf, (int)s, (int)(t - s), buf, (int)s);
                    }
                }
            }
        }
        int size = e.Unpadded > 0 ? e.Unpadded : e.Padded;
        if (size != buf.Length) Array.Resize(ref buf, size);
        return buf;
    }
}
"@

# 法环归档：返回 { Find(路径) → 条目或 $null；Extract(条目) → 字节 }。整份 bhd 逐块解一次要几十秒，只解第一块试公钥
function Open-ErArchives([string]$ErGame, [string]$Smithbox, [string]$ProbeDir) {
    $dll = Get-ChildItem $Smithbox -Recurse -Filter "Andre.Formats.dll" | Select-Object -First 1
    if (-not $dll) { throw "Andre.Formats.dll not found under $Smithbox" }
    [byte[]]$raw = [IO.File]::ReadAllBytes($dll.FullName)
    $found = @{}
    foreach ($align in 0, 1) {
        $count = $raw.Length - $align; $count -= $count % 2
        $text = [Text.Encoding]::Unicode.GetString($raw, $align, $count)
        foreach ($m in [regex]::Matches($text, "MIIB[A-Za-z0-9+/=\r\n]{300,600}")) { $found[($m.Value -replace "[\r\n]", "")] = 1 }
    }
    New-Item -ItemType Directory -Force $ProbeDir | Out-Null
    $probe = Join-Path (Resolve-Path $ProbeDir).Path "bhd_probe.tmp"
    $ctx = @{ Tables = @{}; Bdts = @{}; Entries = @{} }
    try {
        foreach ($file in Get-ChildItem $ErGame -Filter "*.bhd") {
            $stream = [IO.File]::OpenRead($file.FullName); $head = New-Object byte[] 256; [void]$stream.Read($head, 0, 256); $stream.Dispose()
            [IO.File]::WriteAllBytes($probe, $head)
            $key = $null
            foreach ($k in $found.Keys) {
                try {
                    $plain = [NrArchive]::DecryptBhd($probe, $k)
                    if ($plain.Length -ge 4 -and [Text.Encoding]::ASCII.GetString($plain, 0, 4) -eq "BHD5") { $key = $k; break }
                } catch { }
            }
            if (-not $key) { throw "no key opens $($file.FullName)" }
            $name = $file.BaseName
            $ctx.Tables[$name] = [NrArchive]::DecryptBhd($file.FullName, $key)
            $ctx.Bdts[$name] = [IO.Path]::ChangeExtension($file.FullName, ".bdt")
            foreach ($e in [NrArchive]::Parse($ctx.Tables[$name], $name)) { $ctx.Entries[$e.Hash] = $e }
        }
    } finally { Remove-Item $probe -Force -ErrorAction SilentlyContinue }
    $ctx
}
function Get-ErFile($Ctx, [string]$Path) {
    $e = $Ctx.Entries[[NrArchive]::Hash($Path)]
    if (-not $e) { return $null }
    , [NrArchive]::Extract($e, $Ctx.Tables[$e.Archive], $Ctx.Bdts[$e.Archive])
}
