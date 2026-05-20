use alloc::vec::Vec;

/// A null-terminated UTF-16 string slice.
///
/// `WStr` is to [`WString`] as [`str`] is to [`String`]. The internal `[u16]`
/// always ends with a single `0u16` null terminator, and never contains
/// interior null code units.
///
/// Construct a `&'static WStr` at compile time with the [`w!`](crate::w) macro:
///
/// ```ignore
/// let hello: &WStr = w!("Hello, UEFI!");
/// ```
///
/// # Ill-formed UTF-16
///
/// `WStr` permits ill-formed UTF-16 (unpaired surrogates). UEFI firmware
/// occasionally produces such strings. [`WStr::chars`] replaces unpaired
/// surrogates with [`char::REPLACEMENT_CHARACTER`]; all other operations work
/// on the raw code-unit sequence.
#[repr(transparent)]
pub struct WStr {
    inner: [u16],
}

impl WStr {
    /// Creates a `&WStr` from a `u16` slice without any validation.
    ///
    /// # Safety
    ///
    /// - `slice` must end with exactly one `0u16` null terminator.
    /// - `slice` must not contain any `0u16` values before the terminator.
    pub const unsafe fn from_slice_unchecked(slice: &[u16]) -> &Self {
        unsafe { &*(slice as *const [u16] as *const Self) }
    }

    /// Creates a `&WStr` from a `u16` slice, returning `None` if the slice is
    /// not a valid null-terminated string.
    ///
    /// The slice must end with exactly one `0u16` and must contain no interior
    /// null code units. UTF-16 well-formedness (surrogate pairing) is not
    /// checked — see the type-level documentation.
    pub fn from_slice(slice: &[u16]) -> Option<&Self> {
        match slice.split_last() {
            Some((&0, rest)) if !rest.contains(&0) => {
                Some(unsafe { Self::from_slice_unchecked(slice) })
            }
            _ => None,
        }
    }

    /// Creates a `&WStr` from a null-terminated UTF-16 pointer.
    ///
    /// # Safety
    ///
    /// - `ptr` must be non-null and point to a null-terminated sequence of
    ///   `u16` values.
    /// - The memory must remain valid and immutable for the entire lifetime
    ///   `'a`. The compiler cannot verify this; incorrect lifetimes (e.g.
    ///   `'static` on a stack buffer) cause undefined behaviour.
    pub const unsafe fn from_ptr<'a>(ptr: *const u16) -> &'a Self {
        unsafe {
            #[inline]
            const unsafe fn strlen(ptr: *const u16) -> usize {
                let mut len = 0;
                while unsafe { *ptr.add(len) } != 0u16 {
                    len += 1;
                }
                len
            }
            let len = strlen(ptr);
            Self::from_slice_unchecked(core::slice::from_raw_parts(ptr, len + 1))
        }
    }

    /// Returns the underlying `u16` slice **including** the null terminator.
    ///
    /// The last element is always `0u16`. Do not pass this slice to APIs that
    /// expect a string without a terminator; use
    /// [`as_slice_without_nul`](Self::as_slice_without_nul) instead. To obtain
    /// a raw pointer for UEFI calls, use [`as_ptr`](Self::as_ptr).
    pub const fn as_slice(&self) -> &[u16] {
        &self.inner
    }

    /// Returns the underlying `u16` slice **without** the null terminator.
    pub fn as_slice_without_nul(&self) -> &[u16] {
        // SAFETY: invariant guarantees at least one element (the null terminator)
        unsafe { self.inner.get_unchecked(..self.inner.len() - 1) }
    }

    /// Returns the number of UTF-16 code units, not counting the null
    /// terminator.
    ///
    /// This counts code units, not characters: supplementary-plane characters
    /// encoded as surrogate pairs count as two.
    pub fn len(&self) -> usize {
        self.inner.len() - 1
    }

    /// Returns `true` if this string contains no code units.
    pub fn is_empty(&self) -> bool {
        self.inner.len() == 1
    }

    /// Returns a raw pointer to the first code unit (the start of the
    /// null-terminated string). Suitable for passing directly to UEFI APIs.
    pub const fn as_ptr(&self) -> *const u16 {
        self.inner.as_ptr()
    }

    /// Returns an iterator over the [`char`] values of this string.
    ///
    /// Surrogate pairs are decoded into their supplementary-plane code points.
    /// Unpaired surrogates are replaced with [`char::REPLACEMENT_CHARACTER`]
    /// (U+FFFD).
    pub fn chars(&self) -> Chars<'_> {
        Chars {
            slice: self.as_slice_without_nul(),
        }
    }

    /// Returns an iterator over `(code_unit_index, char)` pairs.
    ///
    /// The index is the zero-based offset of the first code unit of the
    /// character within [`as_slice_without_nul`](Self::as_slice_without_nul).
    /// Surrogate pairs advance the index by 2; all other code units by 1.
    pub fn char_indices(&self) -> CharIndices<'_> {
        CharIndices {
            slice: self.as_slice_without_nul(),
            offset: 0,
        }
    }

    /// Allocates a new [`WString`] with the same contents as `self`.
    pub fn to_wstring(&self) -> WString {
        WString::from(self)
    }
}

// ── Equality ─────────────────────────────────────────────────────────────────

impl PartialEq for WStr {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
impl Eq for WStr {}

impl PartialEq<str> for WStr {
    /// Compares code points decoded from both strings without allocating.
    fn eq(&self, other: &str) -> bool {
        let mut wc = self.chars();
        let mut uc = other.chars();
        loop {
            match (wc.next(), uc.next()) {
                (Some(a), Some(b)) => {
                    if a != b {
                        return false;
                    }
                }
                (None, None) => return true,
                _ => return false,
            }
        }
    }
}

impl PartialEq<WStr> for str {
    fn eq(&self, other: &WStr) -> bool {
        other == self
    }
}

// ── Ordering ──────────────────────────────────────────────────────────────────

impl PartialOrd for WStr {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Ordering is lexicographic by UTF-16 **code unit** value.
///
/// # Footgun: supplementary-plane characters sort out of code-point order
///
/// Surrogate high values (`0xD800`–`0xDBFF`) are numerically less than
/// `0xE000`, so a supplementary character like U+10000 (encoded
/// `[0xD800, 0xDC00]`) sorts *before* U+E000 even though U+10000 > U+E000 by
/// code point. For BMP-only strings — the vast majority of UEFI strings — this
/// makes no practical difference.
impl Ord for WStr {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_slice_without_nul()
            .cmp(other.as_slice_without_nul())
    }
}

// ── Hashing ───────────────────────────────────────────────────────────────────

impl core::hash::Hash for WStr {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // Include the null so that the hash value is stable and unique.
        self.inner.hash(state);
    }
}

// ── Formatting ────────────────────────────────────────────────────────────────

impl core::fmt::Display for WStr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use core::fmt::Write;
        for c in self.chars() {
            f.write_char(c)?;
        }
        Ok(())
    }
}

impl core::fmt::Debug for WStr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use core::fmt::Write;
        f.write_char('"')?;
        for c in self.chars() {
            for escaped in c.escape_debug() {
                f.write_char(escaped)?;
            }
        }
        f.write_char('"')
    }
}

// ── Chars iterator ────────────────────────────────────────────────────────────

/// Iterator over the [`char`] values of a [`WStr`].
///
/// Created by [`WStr::chars`]. Decodes UTF-16 surrogate pairs; unpaired
/// surrogates yield [`char::REPLACEMENT_CHARACTER`].
pub struct Chars<'a> {
    slice: &'a [u16],
}

impl Iterator for Chars<'_> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        let (&unit, rest) = self.slice.split_first()?;
        self.slice = rest;

        if !(0xD800..=0xDFFF).contains(&unit) {
            // SAFETY: non-surrogate BMP code units are valid Unicode scalar values
            return Some(unsafe { char::from_u32_unchecked(unit as u32) });
        }

        if (0xD800..=0xDBFF).contains(&unit)
            && let Some((&low, rest2)) = rest.split_first()
            && (0xDC00..=0xDFFF).contains(&low)
        {
            self.slice = rest2;
            let codepoint = 0x10000u32 + ((unit as u32 - 0xD800) << 10) + (low as u32 - 0xDC00);
            // SAFETY: valid surrogate pairs always decode to U+10000..=U+10FFFF
            return Some(unsafe { char::from_u32_unchecked(codepoint) });
        }

        Some(char::REPLACEMENT_CHARACTER)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.slice.len();
        // Minimum: every two code units form a surrogate pair → ceil(n/2) chars.
        // Maximum: every code unit is a BMP character → n chars.
        (n.div_ceil(2), Some(n))
    }
}

impl core::iter::FusedIterator for Chars<'_> {}

// ── CharIndices iterator ──────────────────────────────────────────────────────

/// Iterator over `(code_unit_index, char)` pairs of a [`WStr`].
///
/// Created by [`WStr::char_indices`].
pub struct CharIndices<'a> {
    slice: &'a [u16],
    offset: usize,
}

impl Iterator for CharIndices<'_> {
    type Item = (usize, char);

    fn next(&mut self) -> Option<(usize, char)> {
        let (&unit, rest) = self.slice.split_first()?;
        let idx = self.offset;

        if !(0xD800..=0xDFFF).contains(&unit) {
            self.slice = rest;
            self.offset += 1;
            // SAFETY: non-surrogate BMP code units are valid Unicode scalar values
            return Some((idx, unsafe { char::from_u32_unchecked(unit as u32) }));
        }

        if (0xD800..=0xDBFF).contains(&unit)
            && let Some((&low, rest2)) = rest.split_first()
            && (0xDC00..=0xDFFF).contains(&low)
        {
            self.slice = rest2;
            self.offset += 2;
            let cp = 0x10000u32 + ((unit as u32 - 0xD800) << 10) + (low as u32 - 0xDC00);
            // SAFETY: valid surrogate pairs always decode to U+10000..=U+10FFFF
            return Some((idx, unsafe { char::from_u32_unchecked(cp) }));
        }

        self.slice = rest;
        self.offset += 1;
        Some((idx, char::REPLACEMENT_CHARACTER))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.slice.len();
        (n.div_ceil(2), Some(n))
    }
}

impl core::iter::FusedIterator for CharIndices<'_> {}

// ── WString ───────────────────────────────────────────────────────────────────

/// An owned, heap-allocated null-terminated UTF-16 string.
///
/// `WString` is to [`WStr`] as [`String`] is to [`str`]. The internal
/// `Vec<u16>` always ends with a `0u16` null terminator and never contains
/// interior null code units.
///
/// `WString` implements [`core::fmt::Write`], so the `write!` macro works:
///
/// ```ignore
/// use core::fmt::Write;
/// let mut s = WString::new();
/// write!(s, "boot option {}", index).unwrap();
/// ```
pub struct WString {
    inner: Vec<u16>,
}

impl WString {
    /// Creates a new, empty `WString`.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: alloc::vec![0u16],
        }
    }

    /// Creates a new, empty `WString` pre-allocated for at least `cap` code
    /// units (not counting the null terminator).
    pub fn with_capacity(cap: usize) -> Self {
        let mut inner = Vec::with_capacity(cap + 1);
        inner.push(0u16);
        Self { inner }
    }

    /// Returns the number of UTF-16 code units, not counting the null
    /// terminator.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len() - 1
    }

    /// Returns `true` if this string contains no code units.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.len() == 1
    }

    /// Returns the number of code units this string can hold without
    /// reallocating, not counting the null terminator slot.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.inner.capacity().saturating_sub(1)
    }

    /// Reserves capacity for at least `additional` more code units beyond the
    /// current length.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    /// Shrinks the allocation to fit the current content.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
    }

    /// Returns a borrowed [`&WStr`](WStr).
    #[inline]
    pub fn as_wstr(&self) -> &WStr {
        // SAFETY: inner always ends with exactly one 0u16 and has no interior nulls
        unsafe { WStr::from_slice_unchecked(&self.inner) }
    }

    /// Appends the UTF-8 string `s`, re-encoding it as UTF-16.
    pub fn push_str(&mut self, s: &str) {
        let _ = self.inner.pop(); // remove null terminator
        self.inner.extend(s.encode_utf16());
        self.inner.push(0u16);
    }

    /// Appends the contents of another [`WStr`].
    pub fn push_wstr(&mut self, s: &WStr) {
        let _ = self.inner.pop(); // remove null terminator
        self.inner.extend_from_slice(s.as_slice_without_nul());
        self.inner.push(0u16);
    }

    /// Appends a single [`char`], encoding it as UTF-16.
    pub fn push(&mut self, c: char) {
        let _ = self.inner.pop(); // remove null terminator
        let mut buf = [0u16; 2];
        self.inner.extend_from_slice(c.encode_utf16(&mut buf));
        self.inner.push(0u16);
    }

    /// Removes and returns the last character.
    ///
    /// Returns `None` if the string is empty. A surrogate pair is removed as a
    /// unit and decoded to its supplementary-plane [`char`]. A lone surrogate
    /// is removed and yields [`char::REPLACEMENT_CHARACTER`].
    pub fn pop(&mut self) -> Option<char> {
        let len = self.inner.len(); // always includes the null
        if len <= 1 {
            return None;
        }

        let last = self.inner[len - 2]; // last code unit before null

        let (c, units) = if (0xDC00..=0xDFFF).contains(&last) && len >= 3 {
            let prev = self.inner[len - 3];
            if (0xD800..=0xDBFF).contains(&prev) {
                let cp =
                    0x10000u32 + ((prev as u32 - 0xD800) << 10) + (last as u32 - 0xDC00);
                // SAFETY: valid surrogate pair
                (unsafe { char::from_u32_unchecked(cp) }, 2usize)
            } else {
                (char::REPLACEMENT_CHARACTER, 1)
            }
        } else if (0xD800..=0xDBFF).contains(&last) {
            (char::REPLACEMENT_CHARACTER, 1)
        } else {
            // SAFETY: non-surrogate BMP code unit
            (unsafe { char::from_u32_unchecked(last as u32) }, 1)
        };

        // Place the new null terminator where the removed char started, then
        // truncate to remove the trailing code unit(s) plus the old null.
        self.inner[len - 1 - units] = 0u16;
        self.inner.truncate(len - units);
        Some(c)
    }

    /// Truncates the string to `n` UTF-16 code units.
    ///
    /// If `n >= self.len()`, this is a no-op. If `n` would split a surrogate
    /// pair, the pair is removed entirely (truncation happens at `n - 1`).
    pub fn truncate(&mut self, mut n: usize) {
        if n >= self.len() {
            return;
        }
        // Back up one if we would split a surrogate pair.
        if n > 0
            && (0xDC00..=0xDFFF).contains(&self.inner[n])
            && (0xD800..=0xDBFF).contains(&self.inner[n - 1])
        {
            n -= 1;
        }
        self.inner[n] = 0u16;
        self.inner.truncate(n + 1);
    }

    /// Clears the string, removing all code units.
    ///
    /// Retains the underlying allocation; call
    /// [`shrink_to_fit`](Self::shrink_to_fit) afterward to release memory.
    pub fn clear(&mut self) {
        self.inner.truncate(1);
        self.inner[0] = 0u16;
    }

    /// Constructs a `WString` by taking ownership of a `Vec<u16>`, validating
    /// its contents.
    ///
    /// The vector must end with exactly one `0u16` and contain no interior
    /// null code units. Returns `None` if invalid; the vector is consumed in
    /// either case.
    pub fn from_vec(v: Vec<u16>) -> Option<Self> {
        if WStr::from_slice(&v).is_some() {
            Some(Self { inner: v })
        } else {
            None
        }
    }

    /// Consumes this `WString` and returns a raw pointer to the
    /// null-terminated UTF-16 buffer.
    ///
    /// The caller must eventually reclaim ownership via [`WString::from_raw`]
    /// to free the allocation.
    ///
    /// For Rust-to-Rust roundtrips, prefer
    /// [`into_raw_parts`](Self::into_raw_parts) which preserves the exact
    /// `(pointer, length, capacity)` triple and does not require
    /// `shrink_to_fit`.
    pub fn into_raw(self) -> *mut u16 {
        let mut v = self.inner;
        v.shrink_to_fit(); // ensure capacity == len so from_raw can reconstruct
        let ptr = v.as_mut_ptr();
        core::mem::forget(v);
        ptr
    }

    /// Reclaims a `WString` from a raw pointer previously returned by
    /// [`WString::into_raw`].
    ///
    /// # Safety
    ///
    /// - `ptr` must have been produced by [`WString::into_raw`]. Do **not**
    ///   pass pointers obtained from firmware or other sources: use
    ///   [`WStr::from_ptr`] to borrow those instead.
    /// - The buffer must still be a valid null-terminated UTF-16 string with
    ///   no interior null code units.
    /// - `ptr` must not be aliased or used after this call.
    pub unsafe fn from_raw(ptr: *mut u16) -> Self {
        unsafe {
            let mut len = 0usize;
            while *ptr.add(len) != 0u16 {
                len += 1;
            }
            let cap = len + 1;
            Self {
                inner: Vec::from_raw_parts(ptr, cap, cap),
            }
        }
    }

    /// Decomposes this `WString` into its raw components `(pointer, length,
    /// capacity)`.
    ///
    /// Both `length` and `capacity` include the null terminator slot. Pass all
    /// three values to [`from_raw_parts`](Self::from_raw_parts) to reconstruct
    /// the string.
    pub fn into_raw_parts(self) -> (*mut u16, usize, usize) {
        let mut v = self.inner;
        let len = v.len();
        let cap = v.capacity();
        let ptr = v.as_mut_ptr();
        core::mem::forget(v);
        (ptr, len, cap)
    }

    /// Reconstructs a `WString` from the raw components returned by
    /// [`into_raw_parts`](Self::into_raw_parts).
    ///
    /// # Safety
    ///
    /// - `ptr`, `len`, `cap` must have been returned by a single call to
    ///   `into_raw_parts` on this type.
    /// - The buffer must still be a valid null-terminated UTF-16 string with
    ///   no interior null code units.
    /// - `ptr` must not be aliased or used after this call.
    pub unsafe fn from_raw_parts(ptr: *mut u16, len: usize, cap: usize) -> Self {
        unsafe {
            Self {
                inner: Vec::from_raw_parts(ptr, len, cap),
            }
        }
    }

    /// Consumes this `WString` and returns the underlying `Vec<u16>`
    /// **including** the null terminator.
    pub fn into_vec(self) -> Vec<u16> {
        self.inner
    }
}

// ── Standard trait impls for WString ─────────────────────────────────────────

impl Default for WString {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for WString {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl core::ops::Deref for WString {
    type Target = WStr;
    fn deref(&self) -> &WStr {
        self.as_wstr()
    }
}

impl AsRef<WStr> for WString {
    fn as_ref(&self) -> &WStr {
        self.as_wstr()
    }
}

impl core::borrow::Borrow<WStr> for WString {
    fn borrow(&self) -> &WStr {
        self.as_wstr()
    }
}

// ── Conversions ───────────────────────────────────────────────────────────────

impl From<&str> for WString {
    fn from(s: &str) -> Self {
        let mut ws = Self::with_capacity(s.len());
        ws.push_str(s);
        ws
    }
}

impl From<char> for WString {
    fn from(c: char) -> Self {
        let mut ws = Self::with_capacity(2);
        ws.push(c);
        ws
    }
}

impl From<&WStr> for WString {
    fn from(s: &WStr) -> Self {
        let mut inner = Vec::with_capacity(s.inner.len());
        inner.extend_from_slice(&s.inner);
        Self { inner }
    }
}

impl From<WString> for Vec<u16> {
    fn from(s: WString) -> Vec<u16> {
        s.into_vec()
    }
}

impl TryFrom<Vec<u16>> for WString {
    type Error = Vec<u16>;

    fn try_from(v: Vec<u16>) -> core::result::Result<Self, Vec<u16>> {
        if WStr::from_slice(&v).is_some() {
            Ok(Self { inner: v })
        } else {
            Err(v)
        }
    }
}

// ── Equality ──────────────────────────────────────────────────────────────────

impl PartialEq for WString {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
impl Eq for WString {}

impl PartialEq<WStr> for WString {
    fn eq(&self, other: &WStr) -> bool {
        self.inner.as_slice() == &other.inner
    }
}

impl PartialEq<WString> for WStr {
    fn eq(&self, other: &WString) -> bool {
        &self.inner == other.inner.as_slice()
    }
}

impl PartialEq<str> for WString {
    fn eq(&self, other: &str) -> bool {
        self.as_wstr() == other
    }
}

impl PartialEq<WString> for str {
    fn eq(&self, other: &WString) -> bool {
        other == self
    }
}

// ── Ordering ──────────────────────────────────────────────────────────────────

impl PartialOrd for WString {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WString {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (**self).cmp(&**other)
    }
}

impl PartialOrd<WStr> for WString {
    fn partial_cmp(&self, other: &WStr) -> Option<core::cmp::Ordering> {
        (**self).partial_cmp(other)
    }
}

impl PartialOrd<WString> for WStr {
    fn partial_cmp(&self, other: &WString) -> Option<core::cmp::Ordering> {
        self.partial_cmp(other.as_wstr())
    }
}

// ── Hashing ───────────────────────────────────────────────────────────────────

impl core::hash::Hash for WString {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // Delegate through WStr so that HashMap<WString> lookups with &WStr
        // keys work — required by the Borrow<WStr> contract.
        (**self).hash(state);
    }
}

// ── Formatting ────────────────────────────────────────────────────────────────

impl core::fmt::Display for WString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self.as_wstr(), f)
    }
}

impl core::fmt::Debug for WString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_wstr(), f)
    }
}

impl core::fmt::Write for WString {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.push_str(s);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        self.push(c);
        Ok(())
    }
}

// ── Extend / FromIterator ─────────────────────────────────────────────────────

impl Extend<char> for WString {
    fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) {
        self.inner.pop(); // remove null once for the whole batch
        let mut buf = [0u16; 2];
        for c in iter {
            self.inner.extend_from_slice(c.encode_utf16(&mut buf));
        }
        self.inner.push(0u16);
    }
}

impl<'a> Extend<&'a str> for WString {
    fn extend<I: IntoIterator<Item = &'a str>>(&mut self, iter: I) {
        self.inner.pop();
        for s in iter {
            self.inner.extend(s.encode_utf16());
        }
        self.inner.push(0u16);
    }
}

impl<'a> Extend<&'a WStr> for WString {
    fn extend<I: IntoIterator<Item = &'a WStr>>(&mut self, iter: I) {
        self.inner.pop();
        for s in iter {
            self.inner.extend_from_slice(s.as_slice_without_nul());
        }
        self.inner.push(0u16);
    }
}

impl FromIterator<char> for WString {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        let mut s = WString::new();
        s.extend(iter);
        s
    }
}
