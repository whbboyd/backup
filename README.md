A checksum-based incremental backup utility.

This was written to take incremental backups of a Minecraft world, which was very large, mostly static, and for which I was unsure how frequently the server touched files without changing them, making data-based change detection preferable to existing timestamp-based incremental backup tools.
