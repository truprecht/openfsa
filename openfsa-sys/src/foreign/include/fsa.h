enum fsa_type {
    STDVECTOR,
    RMEPSILON,
    INTERSECTION,
    DIFFERENCE,
    COMPACT,
    CONST
};

enum vec_type {
    CHAR,
    INT,
    ARC
};

struct fsa_t {
    unsigned char type;
    void *fsa;
};

struct fsa_arc {
    int from_state, to_state, label;
    float weight;
};

struct vec_t {
    unsigned char type;
    void *vec_obj, *first;
    size_t length;
};

#ifdef __cplusplus
extern "C" {
#endif

    struct fsa_t fsa_from_string(const struct vec_t *vec);
    struct vec_t fsa_to_string(const struct fsa_t *f);

    struct fsa_t fsa_from_arc_list(int states, const struct vec_t *final_states,  const struct vec_t *arclist);
    struct vec_t fsa_to_arc_list(const struct fsa_t *fsa);

    int fsa_initial_state(const struct fsa_t *fsa);
    struct vec_t fsa_final_states(const struct fsa_t *fsa);

    struct fsa_t fsa_n_best(const struct fsa_t *fsa, int n);
    struct fsa_t fsa_intersect(const struct fsa_t *a, const struct fsa_t *b);
    struct fsa_t fsa_difference(const struct fsa_t *a, const struct fsa_t *b);

    void fsa_free(const struct fsa_t *fsa);
    void vec_free(const struct vec_t *vec);

#ifdef __cplusplus
}
#endif