A checksum-based incremental backup utility.

This was written to take incremental backups of a Minecraft world, which was very large, mostly static, and for which I was unsure how frequently the server touched files without changing them, making data-based change detection preferable to existing timestamp-based incremental backup tools.

Note: this project can currently be built by `rustc` no later than 1.22 due to a backwards-incompatible change to "error[E0642]: patterns aren't allowed in methods without bodies" in 1.23.

