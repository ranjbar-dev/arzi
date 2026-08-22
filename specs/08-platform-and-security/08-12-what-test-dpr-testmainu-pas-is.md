_Part of [08-platform-and-security](../08-platform-and-security.md) — [index](00-index.md)_

## 12. What `test.dpr` / `testmainU.pas` is

**`test.dpr` is the licence-key generator, shipped as a standalone tool.**

`test.dpr` (`test.dpr:1-17`) is a four-unit program:

```pascal
program test;
uses Vcl.Forms, testmainU in 'testmainU.pas' {Form4},
     LockUnit in 'LockUnit.pas', INI in 'INI.pas', Utility in 'Utility.pas';
begin
  Application.Initialize;
  Application.MainFormOnTaskbar := False;
  Application.CreateForm(TTestF, TestF);
  Application.Run;
end.
```

No data module, no database, no main application — just `TTestF`.

`testmainU.pas` defines `TTestF` with five read-only "system" edits (`Sytem1..5` — note
the misspelling), five "result" edits (`Result1..5`), a hidden `Pass` edit, and three
buttons.

**It has two roles, and it is compiled into *both* executables:**

| Role | Where | How |
|---|---|---|
| **Activation dialog** (customer side) | inside `arzi.exe` — `arzi.dpr:46`, created at `arzi.dpr:210`, invoked from `Mainu.pas:902` when the licence test fails | `init` (`:107-119`) recomputes the five machine numbers via `MakeD`, blanks the five response fields for the customer to type in, and shows `B_Calc` **only if `dm.admin`** (`:117`). `Button1Click` (`:53-65`) hashes the five typed responses (`MakeI`, `:162-171`) and writes the result to `[Base] CS3`, then reports `رمز با موفقیت ثبت شد` ("key registered successfully"). |
| **Key generator** (vendor side) | `test.exe` (10.7 MB, present in the repo) and, latently, `arzi.exe` | `B_CalcClick` (`:79-100`) first reveals the hidden password box, then requires `Util.Encrypt(Pass.Text)` to equal a hard-coded literal; on success calls `MakeR` (`:173-211`), which fills `Result1..5` from the five machine numbers. `Button1MouseUp` also triggers it if Ctrl is held (`:67-72`). |

`test.INI` in the repo is a sample settings file containing a single `[Base] CS3` value —
i.e. one generated licence.

**Implications for the rebuild:**

1. `test.dpr` is **not** a test suite and contains no tests. There is **no automated
   test coverage anywhere in this project.** The rebuild starts from zero on tests.
2. The generator and the validator share the same code and the same non-secret
   algorithm; the "secret" is a password literal in `testmainU.pas:93`.
3. Nothing in `test.dpr` should be carried over. Licensing becomes a server-side
   subscription/entitlement check (§7.5), and `test.dpr` is deleted.
4. ⚠️ Before deleting, confirm whether the customer currently uses `test.exe` as part of
   their sales/support process; if so, that business process has to be replaced too.

---


---

Prev: [11. Concurrency and multi-user behaviour](08-11-concurrency-and-multi-user-behaviour.md) · Next: [13. Open questions](08-13-open-questions.md)
