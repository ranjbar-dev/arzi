// Step 1.6 (docs/phase-1-platform-and-auth.md §1.6): "seed the fa locale with
// the Persian captions already captured verbatim throughout specs/" — every
// caption below is copied from a spec table, not re-translated by hand.
// Domain nav labels: specs/01-glossary.md §§1-4 section headings +
// specs/08-platform-and-security/08-01-the-complete-main-menu-tree.md tab
// captions. Admin captions: 08-04-authorization.md §4.3's Admin.pas replacement.
export const fa = {
  common: {
    appName: "آرزی",
    save: "ذخیره",
    cancel: "انصراف",
    close: "بستن",
    confirm: "تایید",
    create: "ایجاد",
    edit: "ویرایش",
    delete: "حذف",
    loading: "در حال بارگذاری…",
    error: "خطا",
    actions: "عملیات",
    yes: "بله",
    no: "خیر",
    active: "فعال",
    inactive: "غیرفعال",
    comingSoon: "این بخش در فازهای بعدی ساخته می‌شود.",
  },
  nav: {
    dashboard: "خانه",
    accounting: "حسابداری",
    inventory: "انبار",
    treasury: "خزانه",
    parties: "اشخاص",
    reporting: "گزارش‌ها",
    platform: "تنظیمات",
  },
  auth: {
    tenantSlug: "شناسه سازمان",
    username: "نام کاربری",
    password: "کلمه عبور",
    login: "ورود",
    loggingIn: "در حال ورود…",
    logout: "خروج",
    invalidCredentials: "نام کاربری، کلمه عبور یا سازمان نادرست است.",
    tooManyAttempts: "تعداد تلاش‌ها بیش از حد مجاز است. کمی بعد دوباره تلاش کنید.",
    changePassword: "تغییر رمز",
    currentPassword: "کلمه عبور فعلی",
    newPassword: "کلمه عبور جدید",
  },
  shell: {
    tenant: "سازمان",
    fiscalYear: "سال مالی",
    noFiscalYear: "سال مالی تعیین نشده",
    user: "کاربر",
    superuser: "مدیر ارشد",
  },
  admin: {
    // Admin.pas replacement (08-04-authorization.md §4.3's grid-of-checkboxes).
    users: "کاربران",
    newUser: "کاربر جدید",
    createUser: "ایجاد کاربر",
    enableUser: "فعال‌سازی",
    disableUser: "غیرفعال‌سازی",
    setPassword: "تعیین کلمه عبور",
    permissions: "دسترسی‌ها",
    grantPermissions: "ذخیره دسترسی‌ها",
    usernameTaken: "این نام کاربری قبلاً استفاده شده است.",
    passwordTooShort: "کلمه عبور باید حداقل ۸ نویسه باشد.",
  },
  fiscalYears: {
    // 08-01-the-complete-main-menu-tree.md §1.6 "تغيير سال مالي".
    title: "سال‌های مالی",
    newFiscalYear: "سال مالی جدید",
    year: "سال",
    startDate: "تاریخ شروع",
    endDate: "تاریخ پایان",
    status: "وضعیت",
    close: "بستن سال مالی",
    switchTo: "انتخاب به‌عنوان سال جاری",
    current: "سال جاری",
    yearAlreadyExists: "سال مالی با این شماره قبلاً ثبت شده است.",
    dateRangeOverlaps: "بازه تاریخ با سال مالی دیگری تداخل دارد.",
    invalidDateRange: "تاریخ پایان باید پس از تاریخ شروع باشد.",
    alreadyClosed: "این سال مالی قبلاً بسته شده است.",
  },
  accounts: {
    // Captions verbatim from specs/03-accounting-core/03-12-a #12.2 (SNewu).
    title: "سرفصلهای حسابداری",
    generalCode: "کد کل",
    subsidiaryCode: "کد معین",
    analyticCode: "کد تفضیل",
    code: "کد حساب",
    fullName: "نام کامل",
    childCount: "زیر شاخه",
    lock: "قفل",
    newCode: "+ جدید",
    editName: "نام",
    editCode: "کد",
    deleteCode: "حذف",
    promote: "ترفیع سطح",
    demote: "تنزیل سطح",
    supplementaryInfo: "اطلاعات تکمیلی",
    enterSubBranch: "ورود به زیر شاخه",
    backToParent: "بازگشت به شاخه بالاتر",
    root: "ریشه",
    codeLabel: "کد",
    nameLabel: "نام",
    newAccountTitle: "ایجاد کد جدید",
    selectAccount: "انتخاب حساب",
    selectTargetParent: "انتخاب شاخه مقصد",
    search: "جستجوی حساب",
    hasChildren: "این کد زیر شاخه دارد و قابل تغییر نیست.",
    duplicateCode: "کد وارد شده تکراری است.",
    invalidCode: "کد باید عددی مثبت باشد.",
    invalidName: "نام کد را وارد کنید.",
    alreadyTopLevel: "این حساب در بالاترین سطح است.",
    alreadyMaxLevel: "این حساب در پایین‌ترین سطح است.",
    maxDepthReached: "این شاخه در عمیق‌ترین سطح است.",
    invalidTargetLevel: "شاخه مقصد باید هم‌سطح باشد.",
    notPostable: "این حساب سطح آخر نیست و قابل ثبت سند نمی‌باشد.",
    postable: "این حساب سطح آخر است و قابل ثبت سند می‌باشد.",
    lockedAccount: "این حساب قفل است.",
  },
} as const;

export type Dictionary = typeof fa;

/** Plain dot-path lookup — usable from Server Components without the
 * react-i18next context (that's for client components, see `I18nProvider`). */
export function t(path: string): string {
  const value = path.split(".").reduce<unknown>((node, key) => {
    if (node && typeof node === "object" && key in node) {
      return (node as Record<string, unknown>)[key];
    }
    return undefined;
  }, fa);
  return typeof value === "string" ? value : path;
}
