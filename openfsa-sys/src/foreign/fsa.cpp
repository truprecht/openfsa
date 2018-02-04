#include <fst/fstlib.h>
#include <vector>
#include <iostream>
#include <sstream>
#include <string>
#include "fsa.h"

const fst::Fst<fst::StdArc>* reinterpret(const struct fsa_t *wrapper) {
    switch(wrapper->type) {
        case STDVECTOR:
            return static_cast<fst::StdVectorFst*>(wrapper->fsa);
        case RMEPSILON:
            return static_cast<fst::RmEpsilonFst<fst::StdArc>*>(wrapper->fsa);
        case INTERSECTION:
            return static_cast<fst::IntersectFst<fst::StdArc>*>(wrapper->fsa);
        case DIFFERENCE:
            return static_cast<fst::DifferenceFst<fst::StdArc>*>(wrapper->fsa);
        case COMPACT:
            return static_cast<fst::CompactFst<fst::StdArc, fst::AcceptorCompactor<fst::StdArc> >*>(wrapper->fsa);
        case CONST:
            return static_cast<fst::ConstFst<fst::StdArc>*>(wrapper->fsa);
    }
    return NULL;
} 

extern "C" {

    struct fsa_t fsa_from_string(const struct vec_t *binary){
        std::istringstream stream;
        std::string binary_string(static_cast<char*>(binary->first), binary->length);
        stream.str(binary_string);

        struct fsa_t wrapper = { 
            COMPACT,
            fst::CompactFst<fst::StdArc, fst::AcceptorCompactor<fst::StdArc> >::Read(stream, fst::FstReadOptions())
        };

        return wrapper;
    }
    
    struct vec_t fsa_to_string(const struct fsa_t *f){
        std::ostringstream stream;
        fst::CompactFst<fst::StdArc, fst::AcceptorCompactor<fst::StdArc> > compactfst(*reinterpret(f));
        
        compactfst.Write(stream, fst::FstWriteOptions());
        std::string binary_string(stream.str());

        // move string's char array into heap with std::vector
        // allocating std::string in heap would give use a const pointer
        std::vector<char> *cstr = new std::vector<char>(binary_string.c_str(), binary_string.c_str() + binary_string.length());

        struct vec_t list = { CHAR, cstr, &((*cstr)[0]), binary_string.length() };
        return list;
    }

    struct fsa_t fsa_from_arc_list( int states
                                  , const struct vec_t *final_states
                                  , const struct vec_t *arc_list){
        
        fst::StdVectorFst mut;
        fsa_arc *arcs = static_cast<fsa_arc*>(arc_list->first);
        int *finals = static_cast<int*>(final_states->first);
        
        // add states
        for (int i = 0; i < states; i++){
            mut.AddState();
        }
        // add arcs
        for (size_t i = 0; i < arc_list->length; i++){
            mut.AddArc(
                arcs[i].from_state, fst::StdArc(arcs[i].label, arcs[i].label, arcs[i].weight, arcs[i].to_state)
            );
        }
        // set final states without weight
        for (size_t i = 0; i < final_states->length; i++){
            mut.SetFinal(finals[i], 0.0);
        }
        // start is always 0
        mut.SetStart(0);

        fst::ArcSort(&mut, fst::ILabelCompare<fst::StdArc>());

        fst::CompactFst<fst::StdArc, fst::AcceptorCompactor<fst::StdArc> > *imut = new fst::CompactFst<fst::StdArc, fst::AcceptorCompactor<fst::StdArc> >(mut);
        
        struct fsa_t wrapper = { COMPACT, imut };
        return wrapper;
    }

    struct vec_t fsa_to_arc_list(const struct fsa_t *wrapper){
        std::vector<struct fsa_arc> *vec = new std::vector<struct fsa_arc>();
        const fst::Fst<fst::StdArc> *fsa = reinterpret(wrapper);
        
        struct fsa_arc carc;
        for (fst::StateIterator<fst::StdFst> state(*fsa); !state.Done(); state.Next()){
            for (fst::ArcIterator<fst::StdFst> arc(*fsa, state.Value()); !arc.Done(); arc.Next()){
                carc.from_state = state.Value();
                carc.to_state = arc.Value().nextstate;
                carc.label = arc.Value().ilabel;
                carc.weight = arc.Value().weight.Value();
                
                vec->push_back(carc);
            }
        }

        struct vec_t al = { ARC, vec, &(*vec)[0], vec->size() };
        return al;
    }

    struct fsa_t fsa_n_best(const struct fsa_t *fsa, int n){
        fst::StdVectorFst nbest;
        fst::ShortestPath(*reinterpret(fsa), &nbest, n);
        fst::RmEpsilon(&nbest);
        
        struct fsa_t ret = { 
            COMPACT,
            new fst::CompactFst<fst::StdArc, fst::AcceptorCompactor<fst::StdArc> >(nbest)
        };
        return ret;
    }

    struct fsa_t fsa_intersect(const struct fsa_t *a, const struct fsa_t *b){
        fst::StdVectorFst inter;
        fst::Intersect(*reinterpret(a), *reinterpret(b), &inter);

        struct fsa_t ret = { 
            COMPACT,
            new fst::CompactFst<fst::StdArc, fst::AcceptorCompactor<fst::StdArc> >(inter)
        };
        return ret;
    }

    struct fsa_t fsa_difference(const struct fsa_t *a, const struct fsa_t *b){
        fst::ArcMapFst<fst::StdArc, fst::StdArc, fst::RmWeightMapper<fst::StdArc> > c(*reinterpret(b), fst::RmWeightMapper<fst::StdArc>());
        fst::DeterminizeFst<fst::StdArc> d(c);
        fst::DifferenceFst<fst::StdArc> difference(*reinterpret(a), d);

        struct fsa_t ret = {
            COMPACT,
            new fst::CompactFst<fst::StdArc, fst::AcceptorCompactor<fst::StdArc> >(difference)
        };
        return ret;
    }

    void fsa_free(const struct fsa_t *fsa){
        delete reinterpret(fsa);
    }

    int fsa_initial_state(const struct fsa_t *fsa){
        return reinterpret(fsa)->Start();
    }

    struct vec_t fsa_final_states(const struct fsa_t *fsa){
        // allocate vector on stack
        std::vector<int> *final_states = new std::vector<int>;
        const fst::Fst<fst::StdArc> *fst = reinterpret(fsa);

        // iterate through states, keep those with final weight ≠ zero
        for (fst::StateIterator<fst::StdFst> state(*fst); !state.Done(); state.Next()) {
            if (fst->Final(state.Value()).Value() != fst::TropicalWeight::Zero()){
                final_states->push_back(state.Value());
            }
        }

        // return list as pointer × length pair
        struct vec_t result = { INT, final_states, &(*final_states)[0], final_states->size() };
        return result;
    }

    void vec_free(const struct vec_t *vec) {
        switch (vec->type) {
            case CHAR:
                delete static_cast<std::vector<char>*>(vec->vec_obj);
                return;
            case INT:
                delete static_cast<std::vector<int>*>(vec->vec_obj);
                return;
            case ARC:
                delete static_cast<std::vector<fsa_arc>*>(vec->vec_obj);
                return;
        }
    }

}