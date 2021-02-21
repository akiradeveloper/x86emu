int abs_f(int x) {
    if (x >= 0) {
        return x;
    } else {
        return -x;
    }
}
int main(void) {
    return abs_f(-3);
}