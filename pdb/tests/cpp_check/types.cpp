// #include <stdint.h>

#include <stdlib.h>

enum EnumSimple
{
    Simple_A = 100,
    Simple_B = 200,
};
typedef EnumSimple EnumSimple;

__declspec(dllexport) EnumSimple g_enumSimpleValue = EnumSimple::Simple_B;

enum class EnumClass
{
    A = 100,
    B = 200,
};

enum EnumOverInt : int
{
    EnumOverInt_A = 100,
    EnumOverInt_B = 200,
};

enum class EnumClassOverInt : int
{
    A = 100,
    B = 200,
};

enum class EnumClassOverUInt8 : unsigned __int8
{
    Z = 10,
};

struct StructWithManyEnums
{
    EnumSimple enum_simple;
    EnumClass enum_class;
    EnumOverInt enum_over_int;
    EnumClassOverInt enum_class_over_int;
    EnumClassOverUInt8 enum_class_over_uint8;
};

struct StructWithPrimitiveTypes
{
#define INT_VARIANTS(ty, name)      \
    ty f_##name;                    \
    ty f_const_##name;              \
    signed ty f_signed_##name;      \
    unsigned ty f_unsigned_##name;  \
    ty *f_##name##_ptr;             \
    const ty *f_const_##name##_ptr; \
    ty *f_##name##_const_ptr;       \
    const ty *f_const_##name##_const_ptr;

    INT_VARIANTS(char, char)
    INT_VARIANTS(int, int)
    INT_VARIANTS(long, long)
    INT_VARIANTS(short, short)
    INT_VARIANTS(long long, _long_long)
    INT_VARIANTS(__int8, int8)
    INT_VARIANTS(__int16, int16)
    INT_VARIANTS(__int32, int32)
    INT_VARIANTS(__int64, int64)

#undef INT_VARIANTS

    void *f_void_ptr;

    bool f_bool;
    bool *f_bool_ptr;

    float f_float;
    float *f_float_ptr;
    double f_double;
    double *f_double_ptr;
};

const int WHAT_IS_SIX_TIMES_SEVEN = 42;

class Zebra
{
public:
    static constexpr short NUMBER_OF_STRIPES = 80;
};

namespace foo
{
    namespace bar
    {
        const long long CONSTANT_INSIDE_NAMESPACE = -333;
    }
}


class __declspec(dllexport) ExportedClass
{
public:
    int x_;
    bool live_;

    ExportedClass();
    ExportedClass(int x);
    ExportedClass(const ExportedClass &other);
    ExportedClass(ExportedClass &&other);
    ExportedClass &operator=(const ExportedClass &other);
    ExportedClass &operator=(ExportedClass &&other);
    operator int() const;
    void operator()() const;
};

ExportedClass::ExportedClass() {
    abort();
}

ExportedClass::ExportedClass(int x) : x_(x), live_(true) {}

ExportedClass::ExportedClass(const ExportedClass &other) : x_(other.x_) {}

ExportedClass::ExportedClass(ExportedClass &&other) : x_(other.x_)
{
    if (other.live_)
    {
        live_ = true;
        x_ = other.x_;
        other.live_ = false;
        other.x_ = 0;
    }
    else
    {
        live_ = false;
        x_ = 0;
    }
}

__declspec(dllexport)
    ExportedClass &ExportedClass::operator=(const ExportedClass &)
{
    return *this;
}

__declspec(dllexport)
    ExportedClass &ExportedClass::operator=(ExportedClass &&)
{
    return *this;
}

__declspec(dllexport)
    ExportedClass::operator int() const
{
    return 0;
}

__declspec(dllexport) void ExportedClass::operator()() const
{
    abort();
}

__declspec(dllexport) ExportedClass *newExportedClass()
{
    return new ExportedClass();
}

__declspec(dllexport) StructWithPrimitiveTypes g_structWithPrimitiveTypes;

__declspec(noinline) int global_function()
{
    return 0;
}

__declspec(noinline) extern "C" int global_function_c_linkage()
{
    return WHAT_IS_SIX_TIMES_SEVEN;
}

__declspec(dllexport) void enums_export(StructWithManyEnums *s)
{
    __annotation(L"Hello!", L"World!");

    s->enum_simple = Simple_A;
    s->enum_class = EnumClass::A;
    s->enum_over_int = EnumOverInt_A;
    s->enum_class_over_int = EnumClassOverInt::A;
    s->enum_class_over_uint8 = EnumClassOverUInt8::Z;

    global_function();
    global_function_c_linkage();
}

